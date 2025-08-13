// Resolve a given hostname to an IP address using `systemd-resolved`'s Varlink service.
// We use the proxy macro to generate a type-safe client API.
use std::{env::args, fmt::Display, net::IpAddr};

use futures_util::{pin_mut, StreamExt};
use serde_repr::{Deserialize_repr, Serialize_repr};
use zlink::{proxy, ReplyError};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut connection = zlink::unix::connect("/run/systemd/resolve/io.systemd.Resolve").await?;

    let args: Vec<_> = args().skip(1).collect();

    // Use pipelining to send all hostname resolution requests at once.
    if args.is_empty() {
        eprintln!("Usage: resolved <hostname> [<hostname> ...]");
        return Ok(());
    }

    // Build the chain of pipelined requests.
    let mut chain = connection.chain_resolve_hostname::<ReplyParams, ReplyError>(&args[0])?;
    for name in &args[1..] {
        chain = chain.resolve_hostname(name)?;
    }

    let replies = chain.send().await?;
    pin_mut!(replies);

    // Collect results and print them.
    let mut i = 0;
    while let Some(reply) = replies.next().await {
        let name = &args[i];
        i += 1;

        match reply? {
            Ok(result) => {
                println!("Results for '{name}':");
                for address in result.into_parameters().unwrap().addresses {
                    println!("\t{address}");
                }
            }
            Err(e) => eprintln!("Error resolving '{name}': {e}"),
        }
    }

    Ok(())
}

#[proxy("io.systemd.Resolve")]
trait ResolvedProxy {
    #[allow(unused)]
    async fn resolve_hostname(
        &mut self,
        name: &str,
    ) -> zlink::Result<Result<ReplyParams<'_>, ReplyError<'_>>>;
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

#[derive(Debug, ReplyError)]
#[zlink(interface = "io.systemd.Resolve")]
enum ReplyError<'e> {
    NoNameServers,
    NoSuchResourceRecord,
    QueryTimedOut,
    MaxAttemptsReached,
    InvalidReply,
    QueryAborted,
    DNSSECValidationFailed {
        #[zlink(rename = "result")]
        _result: &'e str,
        #[zlink(rename = "extendedDNSErrorCode")]
        _extended_dns_error_code: Option<i32>,
        #[zlink(rename = "extendedDNSErrorMessage")]
        _extended_dns_error_message: Option<&'e str>,
    },
    NoTrustAnchor,
    ResourceRecordTypeUnsupported,
    NetworkDown,
    NoSource,
    StubLoop,
    DNSError {
        #[zlink(rename = "rcode")]
        _rcode: i32,
        #[zlink(rename = "extendedDNSErrorCode")]
        _extended_dns_error_code: Option<i32>,
        #[zlink(rename = "extendedDNSErrorMessage")]
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
