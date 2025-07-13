//! Mock systemd-machined service for testing when real systemd services aren't available.

use mayheap::Vec;
use serde::Serialize;
use serde_prefix_all::prefix_all;
use zlink::{
    idl::{self, Comment, EnumVariant, Interface, Parameter, Type::Optional, TypeRef},
    introspect::{ReplyError, Type},
    service::MethodReply,
    varlink_service::{self, Error, Info, InterfaceDescription},
    Call, Service,
};

/// Mock systemd-machined service that serves hardcoded responses.
pub struct MockMachinedService;

impl MockMachinedService {
    /// Create a new mock machined service.
    pub fn new() -> Self {
        Self
    }
}

impl Service for MockMachinedService {
    type MethodCall<'de> = varlink_service::Method<'de>;
    type ReplyParams<'ser> = varlink_service::ReplyParams<'ser>;
    type ReplyStream = futures_util::stream::Empty<zlink::Reply<()>>;
    type ReplyStreamParams = ();
    type ReplyError<'ser> = MockError<'ser>;

    async fn handle<'ser>(
        &'ser mut self,
        call: Call<Self::MethodCall<'_>>,
    ) -> MethodReply<Self::ReplyParams<'ser>, Self::ReplyStream, Self::ReplyError<'ser>> {
        match call.method() {
            varlink_service::Method::GetInfo => {
                // Return hardcoded info that matches the systemd machine service
                let mut interfaces = Vec::new();
                let interface_list = [
                    "io.systemd",
                    "io.systemd.Machine",
                    "io.systemd.MachineImage",
                    "org.varlink.service",
                ];
                for interface in interface_list {
                    interfaces.push(interface).unwrap();
                }

                let info = Info::new(
                    "The systemd Project",
                    "systemd (systemd-machined)",
                    "257.5 (257.5-6.fc42)",
                    "https://systemd.io/",
                    interfaces,
                );

                MethodReply::Single(Some(varlink_service::ReplyParams::Info(info)))
            }
            varlink_service::Method::GetInterfaceDescription { interface } => {
                let description = match *interface {
                    "org.varlink.service" => {
                        InterfaceDescription::from(varlink_service::DESCRIPTION)
                    }
                    "io.systemd.Machine" => InterfaceDescription::from(MACHINE_SERVICE_DESCRIPTION),
                    _ => {
                        return MethodReply::Error(MockError::VarlinkService(
                            Error::InterfaceNotFound {
                                interface: "unknown.interface",
                            },
                        ))
                    }
                };

                MethodReply::Single(Some(varlink_service::ReplyParams::InterfaceDescription(
                    description,
                )))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
#[allow(unused)]
pub enum MockError<'a> {
    VarlinkService(Error<'a>),
    Machined(MachinedError),
}

/// Errors that can be returned by the `io.systemd.Machine` interface.
#[prefix_all("io.systemd.Machine.")]
#[derive(Debug, Clone, PartialEq, Serialize, ReplyError)]
#[zlink(crate = "zlink")]
#[serde(tag = "error", content = "parameters")]
#[allow(unused)]
pub enum MachinedError {
    /// No matching machine currently running.
    NoSuchMachine,
    /// Machine already exists.
    MachineExists,
    /// Machine does not use private networking.
    NoPrivateNetworking,
    /// Machine does not contain OS release information.
    NoOSReleaseInformation,
    /// Machine uses a complex UID/GID mapping, cannot determine shift.
    NoUIDShift,
    /// Requested information is not available.
    NotAvailable,
    /// Requested operation is not supported.
    NotSupported,
    /// There is no IPC service (such as system bus or varlink) in the container.
    NoIPC,
}

/// Interface definition for io.systemd.Machine matching the actual systemd-machined service.
const MACHINE_SERVICE_DESCRIPTION: &Interface<'static> = &{
    const PROCESS_ID_TYPE: &idl::Type<'static> =
        &Optional(TypeRef::new(&idl::Type::Custom("ProcessId")));
    const ACQUIRE_METADATA_TYPE: &idl::Type<'static> =
        &Optional(TypeRef::new(&idl::Type::Custom("AcquireMetadata")));

    // Method definitions with scoped parameters
    const REGISTER_METHOD: &idl::Method<'static> = &{
        const PARAMS: &[&Parameter<'static>] = &[
            &Parameter::new("name", &idl::Type::String, &[]),
            &Parameter::new("id", <Option<String>>::TYPE, &[]),
            &Parameter::new("service", <Option<String>>::TYPE, &[]),
            &Parameter::new("class", &idl::Type::String, &[]),
            &Parameter::new(
                "leader",
                <Option<u32>>::TYPE,
                &[&Comment::new("The leader PID as simple positive integer.")],
            ),
            &Parameter::new(
                "leaderProcessId",
                PROCESS_ID_TYPE,
                &[&Comment::new("The leader PID as ProcessId structure.")],
            ),
            &Parameter::new("rootDirectory", <Option<String>>::TYPE, &[]),
            &Parameter::new("ifIndices", <Option<&[u64]>>::TYPE, &[]),
            &Parameter::new("vSockCid", <Option<u64>>::TYPE, &[]),
            &Parameter::new("sshAddress", <Option<String>>::TYPE, &[]),
            &Parameter::new("sshPrivateKeyPath", <Option<String>>::TYPE, &[]),
            &Parameter::new(
                "allocateUnit",
                <Option<bool>>::TYPE,
                &[&Comment::new(
                    "Controls whether to allocate a scope unit for the machine to register.",
                )],
            ),
            &Parameter::new(
                "allowInteractiveAuthentication",
                <Option<bool>>::TYPE,
                &[&Comment::new("Controls whether interactive authentication (via polkit) shall be allowed. If unspecified defaults to false")]
            ),
        ];
        idl::Method::new("Register", PARAMS, &[], &[])
    };

    const UNREGISTER_METHOD: &idl::Method<'static> = &{
        const PARAMS: &[&Parameter<'static>] = &[
            &Parameter::new(
                "name",
                <Option<String>>::TYPE,
                &[&Comment::new("If non-null the name of a machine")],
            ),
            &Parameter::new(
                "pid",
                PROCESS_ID_TYPE,
                &[&Comment::new("If non-null the PID of a machine. Special value 0 means to take pid of the machine the caller is part of")]
            ),
            &Parameter::new(
                "allowInteractiveAuthentication",
                <Option<bool>>::TYPE,
                &[&Comment::new("Controls whether interactive authentication (via polkit) shall be allowed. If unspecified defaults to false")]
            ),
        ];
        idl::Method::new("Unregister", PARAMS, &[], &[])
    };

    const TERMINATE_METHOD: &idl::Method<'static> = &{
        const PARAMS: &[&Parameter<'static>] = &[
            &Parameter::new(
                "name",
                <Option<String>>::TYPE,
                &[&Comment::new("If non-null the name of a machine")],
            ),
            &Parameter::new(
                "pid",
                PROCESS_ID_TYPE,
                &[&Comment::new("If non-null the PID of a machine. Special value 0 means to take pid of the machine the caller is part of")]
            ),
            &Parameter::new(
                "allowInteractiveAuthentication",
                <Option<bool>>::TYPE,
                &[&Comment::new("Controls whether interactive authentication (via polkit) shall be allowed. If unspecified defaults to false")]
            ),
        ];
        idl::Method::new(
            "Terminate",
            PARAMS,
            &[],
            &[&Comment::new("Terminate machine, killing its processes")],
        )
    };

    const KILL_METHOD: &idl::Method<'static> = &{
        const PARAMS: &[&Parameter<'static>] = &[
            &Parameter::new(
                "name",
                <Option<String>>::TYPE,
                &[&Comment::new("If non-null the name of a machine")],
            ),
            &Parameter::new(
                "pid",
                PROCESS_ID_TYPE,
                &[&Comment::new("If non-null the PID of a machine. Special value 0 means to take pid of the machine the caller is part of")]
            ),
            &Parameter::new(
                "allowInteractiveAuthentication",
                <Option<bool>>::TYPE,
                &[&Comment::new("Controls whether interactive authentication (via polkit) shall be allowed. If unspecified defaults to false")]
            ),
            &Parameter::new(
                "whom",
                <Option<String>>::TYPE,
                &[&Comment::new("Identifier that specifies what precisely to send the signal to (either 'leader' or 'all').")]
            ),
            &Parameter::new(
                "signal",
                &idl::Type::Int,
                &[&Comment::new("Numeric UNIX signal integer.")],
            ),
        ];
        idl::Method::new(
            "Kill",
            PARAMS,
            &[],
            &[&Comment::new(
                "Send a UNIX signal to the machine's processes",
            )],
        )
    };

    const LIST_METHOD: &idl::Method<'static> = &{
        const TIMESTAMP_TYPE: &idl::Type<'static> =
            &Optional(TypeRef::new(&idl::Type::Custom("Timestamp")));
        const PARAMS: &[&Parameter<'static>] = &[
            &Parameter::new(
                "name",
                <Option<String>>::TYPE,
                &[&Comment::new("If non-null the name of a machine")],
            ),
            &Parameter::new(
                "pid",
                PROCESS_ID_TYPE,
                &[&Comment::new("If non-null the PID of a machine. Special value 0 means to take pid of the machine the caller is part of")]
            ),
            &Parameter::new(
                "allowInteractiveAuthentication",
                <Option<bool>>::TYPE,
                &[&Comment::new("Controls whether interactive authentication (via polkit) shall be allowed. If unspecified defaults to false")]
            ),
            &Parameter::new(
                "acquireMetadata",
                ACQUIRE_METADATA_TYPE,
                &[&Comment::new("If 'yes' the output will include machine metadata fields such as 'Addresses', 'OSRelease', and 'UIDShift'. If 'graceful' it's equal to true but gracefully eats up errors")]
            ),
        ];
        const OUTPUT_PARAMS: &[&Parameter<'static>] = &[
            &Parameter::new(
                "name",
                &idl::Type::String,
                &[&Comment::new("Name of the machine")],
            ),
            &Parameter::new(
                "id",
                <Option<String>>::TYPE,
                &[&Comment::new(
                    "128bit ID identifying this machine, formatted in hexadecimal",
                )],
            ),
            &Parameter::new(
                "service",
                <Option<String>>::TYPE,
                &[&Comment::new(
                    "Name of the software that registered this machine",
                )],
            ),
            &Parameter::new(
                "class",
                &idl::Type::String,
                &[&Comment::new("The class of this machine")],
            ),
            &Parameter::new(
                "leader",
                PROCESS_ID_TYPE,
                &[&Comment::new("Leader process PID of this machine")],
            ),
            &Parameter::new(
                "rootDirectory",
                <Option<String>>::TYPE,
                &[&Comment::new(
                    "Root directory of this machine, if known, relative to host file system",
                )],
            ),
            &Parameter::new(
                "unit",
                <Option<String>>::TYPE,
                &[&Comment::new(
                    "The service manager unit this machine resides in",
                )],
            ),
            &Parameter::new(
                "timestamp",
                TIMESTAMP_TYPE,
                &[&Comment::new("Timestamp when the machine was activated")],
            ),
            &Parameter::new(
                "vSockCid",
                <Option<u64>>::TYPE,
                &[&Comment::new(
                    "AF_VSOCK CID of the machine if known and applicable",
                )],
            ),
            &Parameter::new(
                "sshAddress",
                <Option<String>>::TYPE,
                &[&Comment::new("SSH address to connect to")],
            ),
            &Parameter::new(
                "sshPrivateKeyPath",
                <Option<String>>::TYPE,
                &[&Comment::new("Path to private SSH key")],
            ),
            &Parameter::new(
                "addresses",
                <Option<&[&str]>>::TYPE,
                &[&Comment::new(
                    "List of addresses of the machine (simplified)",
                )],
            ),
            &Parameter::new(
                "OSRelease",
                <Option<&[&str]>>::TYPE,
                &[&Comment::new("OS release information of the machine")],
            ),
            &Parameter::new(
                "UIDShift",
                <Option<u64>>::TYPE,
                &[&Comment::new("Return the base UID/GID of the machine")],
            ),
        ];
        idl::Method::new(
            "List",
            PARAMS,
            OUTPUT_PARAMS,
            &[
                &Comment::new("List running machines"),
                &Comment::new("[Supports 'more' flag]"),
            ],
        )
    };

    const OPEN_METHOD: &idl::Method<'static> = &{
        const PARAMS: &[&Parameter<'static>] = &[
            &Parameter::new(
                "name",
                <Option<String>>::TYPE,
                &[&Comment::new("If non-null the name of a machine")],
            ),
            &Parameter::new(
                "pid",
                PROCESS_ID_TYPE,
                &[&Comment::new("If non-null the PID of a machine. Special value 0 means to take pid of the machine the caller is part of")]
            ),
            &Parameter::new(
                "allowInteractiveAuthentication",
                <Option<bool>>::TYPE,
                &[&Comment::new("Controls whether interactive authentication (via polkit) shall be allowed. If unspecified defaults to false")]
            ),
            &Parameter::new(
                "mode",
                &idl::Type::Custom("MachineOpenMode"),
                &[&Comment::new(
                    "There are three possible values: 'tty', 'login', and 'shell'.",
                )],
            ),
            &Parameter::new(
                "user",
                <Option<String>>::TYPE,
                &[&Comment::new(
                    "See description of mode='shell'. Valid only when mode='shell'",
                )],
            ),
            &Parameter::new(
                "path",
                <Option<String>>::TYPE,
                &[&Comment::new(
                    "See description of mode='shell'. Valid only when mode='shell'",
                )],
            ),
            &Parameter::new(
                "args",
                <Option<&[&str]>>::TYPE,
                &[&Comment::new(
                    "See description of mode='shell'. Valid only when mode='shell'",
                )],
            ),
            &Parameter::new(
                "environment",
                <Option<&[&str]>>::TYPE,
                &[&Comment::new(
                    "See description of mode='shell'. Valid only when mode='shell'",
                )],
            ),
        ];
        const OUTPUT_PARAMS: &[&Parameter<'static>] = &[
            &Parameter::new(
                "ptyFileDescriptor",
                &idl::Type::Int,
                &[&Comment::new("File descriptor of the allocated pseudo TTY")],
            ),
            &Parameter::new(
                "ptyPath",
                &idl::Type::String,
                &[&Comment::new("Path to the allocated pseudo TTY")],
            ),
        ];
        idl::Method::new(
            "Open",
            PARAMS,
            OUTPUT_PARAMS,
            &[&Comment::new(
                "Allocates a pseudo TTY in the container in various modes",
            )],
        )
    };

    const METHODS: &[&idl::Method<'static>] = &[
        REGISTER_METHOD,
        UNREGISTER_METHOD,
        TERMINATE_METHOD,
        KILL_METHOD,
        LIST_METHOD,
        OPEN_METHOD,
    ];

    // Custom types
    const CUSTOM_TYPES: &[&idl::CustomType<'static>] = {
        // Enum variants
        const ACQUIRE_METADATA_VARIANTS: &[&EnumVariant<'static>] = &[
            &EnumVariant::new("no", &[]),
            &EnumVariant::new("yes", &[]),
            &EnumVariant::new("graceful", &[]),
        ];

        const MACHINE_OPEN_MODE_VARIANTS: &[&EnumVariant<'static>] = &[
            &EnumVariant::new("tty", &[]),
            &EnumVariant::new("login", &[]),
            &EnumVariant::new("shell", &[]),
        ];
        const PROCESS_ID_FIELDS: &[&idl::Field<'static>] = &[
            &idl::Field::new(
                "pid",
                &idl::Type::Int,
                &[&Comment::new("Numeric UNIX PID value")],
            ),
            &idl::Field::new(
                "pidfdId",
                <Option<u64>>::TYPE,
                &[&Comment::new("64bit inode number of pidfd if known")],
            ),
            &idl::Field::new(
                "bootId",
                <Option<u64>>::TYPE,
                &[&Comment::new(
                    "Boot ID of the system the inode number belongs to",
                )],
            ),
        ];

        const TIMESTAMP_FIELDS: &[&idl::Field<'static>] = &[
            &idl::Field::new(
                "realtime",
                <Option<u64>>::TYPE,
                &[&Comment::new(
                    "Timestamp in µs in the CLOCK_REALTIME clock (wallclock)",
                )],
            ),
            &idl::Field::new(
                "monotonic",
                <Option<u64>>::TYPE,
                &[&Comment::new(
                    "Timestamp in µs in the CLOCK_MONOTONIC clock",
                )],
            ),
        ];

        const ADDRESS_FIELDS: &[&idl::Field<'static>] = &[
            &idl::Field::new("ifindex", <Option<u64>>::TYPE, &[]),
            &idl::Field::new("family", &idl::Type::Int, &[]),
            &idl::Field::new("address", <&[u64]>::TYPE, &[]),
        ];

        // Custom type references
        &[
            &idl::CustomType::Enum(idl::CustomEnum::new("AcquireMetadata", ACQUIRE_METADATA_VARIANTS, &[&Comment::new("A enum field allowing to gracefully get metadata")])),
            &idl::CustomType::Enum(idl::CustomEnum::new("MachineOpenMode", MACHINE_OPEN_MODE_VARIANTS, &[&Comment::new("A enum field which defines way to open TTY for a machine")])),
            &idl::CustomType::Object(idl::CustomObject::new("ProcessId", PROCESS_ID_FIELDS, &[&Comment::new("An object for referencing UNIX processes")])),
            &idl::CustomType::Object(idl::CustomObject::new("Timestamp", TIMESTAMP_FIELDS, &[&Comment::new("A timestamp object consisting of both CLOCK_REALTIME and CLOCK_MONOTONIC timestamps")])),
            &idl::CustomType::Object(idl::CustomObject::new("Address", ADDRESS_FIELDS, &[&Comment::new("An address object")])),
        ]
    };

    Interface::new(
        "io.systemd.Machine",
        METHODS,
        CUSTOM_TYPES,
        MachinedError::VARIANTS,
        &[&Comment::new("systemd machine management interface")],
    )
};
