#![allow(dead_code)]

// Resolve a given hostname to an IP address using `systemd-resolved`'s Varlink service.
// We use the low-level API to send a method call and receive a reply.
use std::{env::args, fmt::Display, net::IpAddr};

use serde_repr::{Deserialize_repr, Serialize_repr};
use zarlink_tokio::unix::Connection;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut connection =
        zarlink_tokio::unix::connect("/run/systemd/resolve/io.systemd.Resolve").await?;

    for name in args().skip(1) {
        for address in resolve(&mut connection, &name)
            .await
            .map_err(|e| e.to_string())?
        {
            println!("{}", address);
        }
    }

    Ok(())
}

async fn resolve<'c>(
    connection: &'c mut Connection,
    name: &str,
) -> Result<Vec<ResolvedAddress>, zarlink::Error<ReplyError<'c>>> {
    // Send out the method call.
    let resolve = Method::ResolveHostName { name: &name };
    connection.send_call(resolve, None, None, None).await?;

    // Receive the reply.
    connection
        .receive_reply::<ReplyParams, ReplyError>()
        .await
        .map(|r| r.into_parameters().addresses)
        .map_err(Into::into)
}

#[derive(Debug, serde::Serialize)]
#[serde(tag = "method", content = "parameters")]
enum Method<'m> {
    #[serde(rename = "io.systemd.Resolve.ResolveHostname")]
    ResolveHostName { name: &'m str },
}

#[derive(Debug, serde::Deserialize)]
struct ReplyParams<'r> {
    addresses: Vec<ResolvedAddress>,
    name: &'r str,
    flags: i32,
}

#[derive(Debug, serde::Deserialize)]
struct ResolvedAddress {
    family: ProtocolFamily,
    address: Vec<u8>,
}

impl Display for ResolvedAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ip = match self.family {
            ProtocolFamily::Inet => {
                let ip = <[u8; 4]>::try_from(self.address.as_slice())
                    .map(IpAddr::from)
                    .unwrap();
                format!("IPv4: {}", ip)
            }
            ProtocolFamily::Inet6 => {
                let ip = <[u8; 16]>::try_from(self.address.as_slice())
                    .map(IpAddr::from)
                    .unwrap();
                format!("IPv6: {}", ip)
            }
            ProtocolFamily::Unspec => {
                format!("Unspecified protocol family: {:?}", self.address)
            }
        };
        write!(f, "{}", ip)
    }
}

#[derive(Serialize_repr, Deserialize_repr, PartialEq, Debug)]
#[repr(u8)]
enum ProtocolFamily {
    Unspec = 0, // Unspecified.
    Inet = 2,   // IP protocol family.
    Inet6 = 10, // IP version 6.
}

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "error", content = "parameters")]
enum ReplyError<'e> {
    #[serde(rename = "io.systemd.Resolve.NoNameServers")]
    NoNameServers,
    #[serde(rename = "io.systemd.Resolve.NoSuchResourceRecord")]
    NoSuchResourceRecord,
    #[serde(rename = "io.systemd.Resolve.QueryTimedOut")]
    QueryTimedOut,
    #[serde(rename = "io.systemd.Resolve.MaxAttemptsReached")]
    MaxAttemptsReached,
    #[serde(rename = "io.systemd.Resolve.InvalidReply")]
    InvalidReply,
    #[serde(rename = "io.systemd.Resolve.QueryAborted")]
    QueryAborted,
    #[serde(rename = "io.systemd.Resolve.DNSSECValidationFailed")]
    DNSSECValidationFailed {
        result: &'e str,
        #[serde(rename = "extendedDNSErrorCode")]
        extended_dns_error_code: Option<i32>,
        #[serde(rename = "extendedDNSErrorMessage")]
        extended_dns_error_message: Option<&'e str>,
    },
    #[serde(rename = "io.systemd.Resolve.NoTrustAnchor")]
    NoTrustAnchor,
    #[serde(rename = "io.systemd.Resolve.ResourceRecordTypeUnsupported")]
    ResourceRecordTypeUnsupported,
    #[serde(rename = "io.systemd.Resolve.NetworkDown")]
    NetworkDown,
    #[serde(rename = "io.systemd.Resolve.NoSource")]
    NoSource,
    #[serde(rename = "io.systemd.Resolve.StubLoop")]
    StubLoop,
    #[serde(rename = "io.systemd.Resolve.DNSError")]
    DNSError {
        rcode: i32,
        #[serde(rename = "extendedDNSErrorCode")]
        extended_dns_error_code: Option<i32>,
        #[serde(rename = "extendedDNSErrorMessage")]
        extended_dns_error_message: Option<&'e str>,
    },
    #[serde(rename = "io.systemd.Resolve.CNAMELoop")]
    CNAMELoop,
    #[serde(rename = "io.systemd.Resolve.BadAddressSize")]
    BadAddressSize,
    #[serde(rename = "io.systemd.Resolve.ResourceRecordTypeInvalidForQuery")]
    ResourceRecordTypeInvalidForQuery,
    #[serde(rename = "io.systemd.Resolve.ZoneTransfersNotPermitted")]
    ZoneTransfersNotPermitted,
    #[serde(rename = "io.systemd.Resolve.ResourceRecordTypeObsolete")]
    ResourceRecordTypeObsolete,
}

impl Display for ReplyError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for ReplyError<'_> {}
