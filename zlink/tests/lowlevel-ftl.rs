use std::{pin::pin, time::Duration};

use futures_util::{pin_mut, stream::StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use serde_prefix_all::prefix_all;
use tokio::{select, time::sleep};
use zlink::{
    notified,
    service::MethodReply,
    unix::{bind, connect},
    Call, Service,
};

#[cfg(feature = "introspection")]
use zlink::introspect::ReplyError;

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn lowlevel_ftl() -> Result<(), Box<dyn std::error::Error>> {
    // Remove the socket file if it exists (from a previous run of this test).
    if let Err(e) = tokio::fs::remove_file(SOCKET_PATH).await {
        // It's OK if the file doesn't exist.
        if e.kind() != std::io::ErrorKind::NotFound {
            return Err(e.into());
        }
    }

    // The transitions between the drive conditions.
    let conditions = [
        DriveCondition {
            state: DriveState::Idle,
            tylium_level: 100,
        },
        DriveCondition {
            state: DriveState::Spooling,
            tylium_level: 90,
        },
        DriveCondition {
            state: DriveState::Spooling,
            tylium_level: 90,
        },
    ];

    // Setup the server and run it in a separate task.
    let listener = bind(SOCKET_PATH).unwrap();
    let service = Ftl::new(conditions[0]);
    let server = zlink::Server::new(listener, service);
    select! {
        res = server.run() => res?,
        res = run_client(&conditions) => res?,
    }

    Ok(())
}

async fn run_client(conditions: &[DriveCondition]) -> Result<(), Box<dyn std::error::Error>> {
    // Now create a client connection that monitor changes in the drive condition.
    let mut conn = connect(SOCKET_PATH).await?;
    let call = Call::new(FtlMethod::GetDriveCondition).set_more(Some(true));
    let mut drive_monitor_stream = pin!(
        conn.chain_call::<FtlMethod, FtlReply, FtlError>(&call)?
            .send()
            .await?
    );

    // And a client that only calls methods.
    {
        let mut conn = connect(SOCKET_PATH).await?;

        // Ask for the drive condition, then set them and then ask again.
        let get_drive_cond = FtlMethod::GetDriveCondition.into();
        let set_drive_cond = FtlMethod::SetDriveCondition {
            condition: conditions[1],
        }
        .into();

        let replies = conn
            .chain_call::<FtlMethod, FtlReply, FtlError>(&get_drive_cond)?
            .append(&set_drive_cond)?
            .append(&get_drive_cond)?
            .send()
            .await?;

        // Now we should be able to get all the replies.
        {
            pin_mut!(replies);

            for i in 0..3 {
                let reply = replies.next().await.unwrap()?.unwrap();
                match reply.into_parameters().unwrap() {
                    FtlReply::DriveCondition(drive_condition) => {
                        assert_eq!(drive_condition, conditions[i]);
                    }
                    _ => panic!("Unexpected reply"),
                }
            }
        }

        let duration = 10;
        let impossible_speed = conditions[1].tylium_level / duration + 1;
        let replies = conn
            // Let's try to jump to a new coordinate but first requiring more tylium
            // than we have.
            .chain_call::<_, FtlReply, FtlError>(
                &FtlMethod::Jump {
                    config: DriveConfiguration {
                        speed: impossible_speed,
                        trajectory: 1,
                        duration: 10,
                    },
                }
                .into(),
            )?
            // Now let's try to jump with a valid speed.
            .append(
                &FtlMethod::Jump {
                    config: DriveConfiguration {
                        speed: impossible_speed - 1,
                        trajectory: 1,
                        duration: 10,
                    },
                }
                .into(),
            )?
            .send()
            .await?;
        pin_mut!(replies);
        let e = replies.try_next().await?.unwrap().unwrap_err();
        // The first call should fail because we didn't have enough energy.
        assert_eq!(e, FtlError::NotEnoughEnergy);

        // The second call should succeed.
        let reply = replies.try_next().await?.unwrap()?;
        assert_eq!(
            reply.parameters(),
            Some(&FtlReply::Coordinates(Coordinate {
                longitude: 1.0,
                latitude: 0.0,
                distance: 10,
            }))
        );
    }

    // `drive_monitor_conn` should have received the drive condition changes.
    let drive_cond = drive_monitor_stream.try_next().await?.unwrap()?;
    match drive_cond.parameters().unwrap() {
        FtlReply::DriveCondition(condition) => {
            assert_eq!(condition, &conditions[1]);
        }
        _ => panic!("Expected DriveCondition reply"),
    }

    Ok(())
}

// The FTL service.
struct Ftl {
    drive_condition: notified::State<DriveCondition, FtlReply>,
    coordinates: Coordinate,
}

impl Ftl {
    fn new(init_conditions: DriveCondition) -> Self {
        Self {
            drive_condition: notified::State::new(init_conditions),
            coordinates: Coordinate {
                longitude: 0.0,
                latitude: 0.0,
                distance: 0,
            },
        }
    }
}

impl Service for Ftl {
    type MethodCall<'de> = FtlMethod;
    type ReplyParams<'ser> = FtlReply;
    type ReplyStream = notified::Stream<Self::ReplyStreamParams>;
    type ReplyStreamParams = FtlReply;
    type ReplyError<'ser> = FtlError;

    async fn handle<'ser>(
        &'ser mut self,
        call: Call<Self::MethodCall<'_>>,
    ) -> MethodReply<Self::ReplyParams<'ser>, Self::ReplyStream, Self::ReplyError<'ser>> {
        match call.method() {
            FtlMethod::GetDriveCondition if call.more().unwrap_or_default() => {
                MethodReply::Multi(self.drive_condition.stream())
            }
            FtlMethod::GetDriveCondition => {
                MethodReply::Single(Some(self.drive_condition.get().into()))
            }
            FtlMethod::SetDriveCondition { condition } => {
                if call.more().unwrap_or_default() {
                    return MethodReply::Error(FtlError::ParameterOutOfRange);
                }
                self.drive_condition.set(*condition);
                MethodReply::Single(Some(self.drive_condition.get().into()))
            }
            FtlMethod::GetCoordinates => {
                MethodReply::Single(Some(FtlReply::Coordinates(self.coordinates)))
            }
            FtlMethod::Jump { config } => {
                if call.more().unwrap_or_default() {
                    return MethodReply::Error(FtlError::ParameterOutOfRange);
                }
                let tylium_required = config.speed * config.duration;
                let mut condition = self.drive_condition.get();
                if tylium_required > condition.tylium_level {
                    return MethodReply::Error(FtlError::NotEnoughEnergy);
                }
                let current_coords = self.coordinates;
                let config = *config;

                sleep(Duration::from_millis(1)).await; // Simulate spooling time.

                let coords = Coordinate {
                    longitude: current_coords.longitude + config.trajectory as f32,
                    latitude: current_coords.latitude,
                    distance: current_coords.distance + config.duration,
                };
                condition.state = DriveState::Idle;
                condition.tylium_level = condition.tylium_level - tylium_required;
                self.drive_condition.set(condition);
                self.coordinates = coords;

                MethodReply::Single(Some(FtlReply::Coordinates(coords)))
            }
        }
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq)]
struct DriveCondition {
    state: DriveState,
    tylium_level: i64,
}

impl From<DriveCondition> for FtlReply {
    fn from(drive_condition: DriveCondition) -> Self {
        FtlReply::DriveCondition(drive_condition)
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DriveState {
    Idle,
    Spooling,
    Busy,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq)]
struct DriveConfiguration {
    speed: i64,
    trajectory: i64,
    duration: i64,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq)]
struct Coordinate {
    longitude: f32,
    latitude: f32,
    distance: i64,
}

impl From<Coordinate> for FtlReply {
    fn from(coordinate: Coordinate) -> Self {
        FtlReply::Coordinates(coordinate)
    }
}

/// The FTL service methods.
#[prefix_all("org.example.ftl.")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "method", content = "parameters")]
enum FtlMethod {
    GetDriveCondition,
    SetDriveCondition { condition: DriveCondition },
    GetCoordinates,
    Jump { config: DriveConfiguration },
}

/// The FTL service replies.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
enum FtlReply {
    DriveCondition(DriveCondition),
    Coordinates(Coordinate),
}

/// The FTL service error replies.
#[prefix_all("org.example.ftl.")]
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "introspection", derive(ReplyError))]
#[serde(tag = "error", content = "parameters")]
enum FtlError {
    NotEnoughEnergy,
    ParameterOutOfRange,
    InvalidCoordinates {
        latitude: f32,
        longitude: f32,
        reason: String,
    },
    SystemOverheat {
        temperature: i32,
    },
}

impl core::fmt::Display for FtlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FtlError::NotEnoughEnergy => write!(f, "Not enough energy"),
            FtlError::ParameterOutOfRange => write!(f, "Parameter out of range"),
            FtlError::InvalidCoordinates {
                latitude,
                longitude,
                reason,
            } => {
                write!(
                    f,
                    "Invalid coordinates ({}, {}): {}",
                    latitude, longitude, reason
                )
            }
            FtlError::SystemOverheat { temperature } => {
                write!(f, "System overheating at {} degrees", temperature)
            }
        }
    }
}

impl std::error::Error for FtlError {}

#[cfg(feature = "introspection")]
#[test_log::test(tokio::test)]
async fn reply_error_derive_works() {
    // Test that the ReplyError derive generates the expected variants.
    assert_eq!(FtlError::VARIANTS.len(), 4);

    // Unit variants
    assert_eq!(FtlError::VARIANTS[0].name(), "NotEnoughEnergy");
    assert!(FtlError::VARIANTS[0].has_no_fields());

    assert_eq!(FtlError::VARIANTS[1].name(), "ParameterOutOfRange");
    assert!(FtlError::VARIANTS[1].has_no_fields());

    // Variant with named fields
    assert_eq!(FtlError::VARIANTS[2].name(), "InvalidCoordinates");
    assert!(!FtlError::VARIANTS[2].has_no_fields());
    let fields: Vec<_> = FtlError::VARIANTS[2].fields().collect();
    assert_eq!(fields.len(), 3);
    assert_eq!(fields[0].name(), "latitude");
    assert_eq!(fields[1].name(), "longitude");
    assert_eq!(fields[2].name(), "reason");

    // Another variant with named fields
    assert_eq!(FtlError::VARIANTS[3].name(), "SystemOverheat");
    assert!(!FtlError::VARIANTS[3].has_no_fields());
    let fields: Vec<_> = FtlError::VARIANTS[3].fields().collect();
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].name(), "temperature");
}

const SOCKET_PATH: &'static str = "/tmp/zlink-lowlevel-ftl.sock";
