use async_broadcast::{Receiver, Sender};
use serde::{Deserialize, Serialize};
use tokio::spawn;
use zlink_tokio::{
    connection::{Call, Reply},
    service::MethodReply,
    unix::{bind, connect},
    Service,
};

#[tokio::test]
async fn lowlevel_ftl() -> Result<(), Box<dyn std::error::Error>> {
    // Remove the socket file if it exists (from a previous run of this test).
    tokio::fs::remove_file(SOCKET_PATH).await?;

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
    let server = zlink_tokio::Server::new(listener, service);
    spawn(server.run());

    // Now create a client connection that monitor changes in the drive condition.
    let mut drive_monitor_conn = connect(SOCKET_PATH).await?;
    drive_monitor_conn
        .send_call(Call::new(Some(Methods::GetDriveCondition)).set_more(Some(true)))
        .await?;

    // And a client that only calls methods.
    {
        let mut conn = connect(SOCKET_PATH).await?;

        // Ask for the drive condition, then set them and then ask again.
        conn.send_call(Methods::GetDriveCondition.into()).await?;
        conn.send_call(
            Methods::SetDriveCondition {
                condition: conditions[1],
            }
            .into(),
        )
        .await?;
        conn.send_call(Methods::GetDriveCondition.into()).await?;

        // Now we should be able to get all the replies.
        for i in 0..3 {
            match conn
                .receive_reply::<Replies, Errors>()
                .await??
                .into_parameters()
                .unwrap()
            {
                Replies::DriveCondition(drive_condition) => {
                    assert_eq!(drive_condition, conditions[i]);
                }
                _ => panic!("Unexpected reply"),
            }
        }
    }

    // `monitor_conn` should received the drive condition changes.
    let drive_cond = drive_monitor_conn
        .receive_reply::<DriveCondition, Errors>()
        .await??
        .into_parameters()
        .unwrap();
    assert_eq!(drive_cond, conditions[1]);

    Ok(())
}

// The FTL service.
struct Ftl {
    drive_condition: DriveCondition,
    drive_condition_channel: (Sender<Reply<Replies>>, Receiver<Reply<Replies>>),
    coordinates: Coordinate,
}

impl Ftl {
    fn new(init_conditions: DriveCondition) -> Self {
        let (mut tx, rx) = async_broadcast::broadcast(1);
        tx.set_overflow(true);
        Self {
            drive_condition: init_conditions,
            drive_condition_channel: (tx, rx),
            coordinates: Coordinate {
                longitude: 0.0,
                latitude: 0.0,
                distance: 0,
            },
        }
    }

    fn set_drive_condition(&mut self, drive_condition: DriveCondition) {
        self.drive_condition = drive_condition;
        self.drive_condition_channel
            .0
            .broadcast_blocking(Replies::DriveCondition(drive_condition).into())
            // We enabled overflow so this can't fail.
            .unwrap();
    }
}

impl Service for Ftl {
    type MethodCall<'de> = Methods;
    type ReplyParams<'ser> = Replies;
    type ReplyStream = Receiver<Reply<Self::ReplyStreamParams>>;
    type ReplyStreamParams = Replies;
    type ReplyError<'ser> = Errors;

    fn handle<'ser>(
        &'ser mut self,
        call: Call<Self::MethodCall<'_>>,
    ) -> MethodReply<Self::ReplyParams<'ser>, Self::ReplyStream, Self::ReplyError<'ser>> {
        match call.method() {
            Methods::GetDriveCondition if call.more().unwrap_or_default() => {
                MethodReply::Multi(self.drive_condition_channel.1.clone())
            }
            Methods::GetDriveCondition => {
                MethodReply::Single(Some(Replies::DriveCondition(self.drive_condition)))
            }
            Methods::SetDriveCondition { condition } => {
                self.set_drive_condition(*condition);
                MethodReply::Single(Some(Replies::DriveCondition(self.drive_condition)))
            }
            Methods::GetCoordinates => {
                MethodReply::Single(Some(Replies::Coordinates(self.coordinates)))
            }
        }
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq)]
struct DriveCondition {
    state: DriveState,
    tylium_level: i64,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DriveState {
    Idle,
    Spooling,
    Busy,
}

/*#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq)]
struct DriveConfiguration {
    speed: i64,
    trajectory: i64,
    duration: i64,
}*/

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq)]
struct Coordinate {
    longitude: f32,
    latitude: f32,
    distance: i64,
}

/// The FTL service methods.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "error", content = "parameters")]
enum Methods {
    #[serde(rename = "org.example.ftl.GetDriveCondition")]
    GetDriveCondition,
    #[serde(rename = "org.example.ftl.SetDriveCondition")]
    SetDriveCondition { condition: DriveCondition },
    #[serde(rename = "org.example.ftl.GetCoordinates")]
    GetCoordinates,
}

/// The FTL service replies.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum Replies {
    DriveCondition(DriveCondition),
    Coordinates(Coordinate),
}

/// The FTL service error replies.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "error", content = "parameters")]
enum Errors {
    #[serde(rename = "org.example.ftl.NotEnoughEnergy")]
    NotEnoughEnergy,
    #[serde(rename = "org.example.ftl.ParameterOutOfRange")]
    ParameterOutOfRange,
}

impl core::fmt::Display for Errors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Errors::NotEnoughEnergy => write!(f, "Not enough energy"),
            Errors::ParameterOutOfRange => write!(f, "Parameter out of range"),
        }
    }
}

impl std::error::Error for Errors {}

const SOCKET_PATH: &'static str = "/tmp/zlink-lowlevel-ftl.sock";
