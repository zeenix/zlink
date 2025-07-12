// Resolve a given hostname to an IP address using `systemd-resolved`'s Varlink service.
// We use the low-level API to send a method call and receive a reply.
use std::{env::args, fmt::Display, net::IpAddr};

use serde_prefix_all::prefix_all;
use serde_repr::{Deserialize_repr, Serialize_repr};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut connection = zlink::unix::connect("/run/systemd/resolve/io.systemd.Resolve").await?;

    let args: Vec<_> = args().skip(1).collect();

    // First send out all the method calls (let's make use of pipelinning feature of Varlink!).
    for name in args.clone() {
        let resolve = Method::ResolveHostname { name: &name };
        connection.enqueue_call(&resolve.into())?;
    }
    connection.flush().await?;

    // Then fetch the results and print them.
    for name in args.clone() {
        match connection
            .receive_reply::<ReplyParams, ReplyError>()
            .await
            .map(|r| r.map(|r| r.into_parameters().unwrap().addresses))?
        {
            Ok(addresses) => {
                println!("Results for '{name}':");
                for address in addresses {
                    println!("\t{address}");
                }
            }
            Err(e) => eprintln!("Error resolving '{name}': {e}"),
        }
    }

    Ok(())
}

#[prefix_all("io.systemd.Resolve.")]
#[derive(Debug, serde::Serialize)]
#[serde(tag = "method", content = "parameters")]
enum Method<'m> {
    ResolveHostname { name: &'m str },
}

#[derive(Debug, serde::Deserialize)]
struct ReplyParams<'r> {
    addresses: Vec<ResolvedAddress>,
    #[serde(rename = "name")]
    _name: &'r str,
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
                format!("IPv4: {ip}")
            }
            ProtocolFamily::Inet6 => {
                let ip = <[u8; 16]>::try_from(self.address.as_slice())
                    .map(IpAddr::from)
                    .unwrap();
                format!("IPv6: {ip}")
            }
            ProtocolFamily::Unspec => {
                format!("Unspecified protocol family: {:?}", self.address)
            }
        };
        write!(f, "{ip}")
    }
}

#[derive(Serialize_repr, Deserialize_repr, PartialEq, Debug)]
#[repr(u8)]
enum ProtocolFamily {
    Unspec = 0, // Unspecified.
    Inet = 2,   // IP protocol family.
    Inet6 = 10, // IP version 6.
}

#[prefix_all("io.systemd.Resolve.")]
#[derive(Debug, serde::Deserialize)]
#[serde(tag = "error", content = "parameters")]
enum ReplyError<'e> {
    NoNameServers,
    NoSuchResourceRecord,
    QueryTimedOut,
    MaxAttemptsReached,
    InvalidReply,
    QueryAborted,
    DNSSECValidationFailed {
        #[serde(rename = "result")]
        _result: &'e str,
        #[serde(rename = "extendedDNSErrorCode")]
        _extended_dns_error_code: Option<i32>,
        #[serde(rename = "extendedDNSErrorMessage")]
        _extended_dns_error_message: Option<&'e str>,
    },
    NoTrustAnchor,
    ResourceRecordTypeUnsupported,
    NetworkDown,
    NoSource,
    StubLoop,
    DNSError {
        #[serde(rename = "rcode")]
        _rcode: i32,
        #[serde(rename = "extendedDNSErrorCode")]
        _extended_dns_error_code: Option<i32>,
        #[serde(rename = "extendedDNSErrorMessage")]
        _extended_dns_error_message: Option<&'e str>,
    },
    CNAMELoop,
    BadAddressSize,
    ResourceRecordTypeInvalidForQuery,
    ZoneTransfersNotPermitted,
    ResourceRecordTypeObsolete,
}

impl Display for ReplyError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for ReplyError<'_> {}
