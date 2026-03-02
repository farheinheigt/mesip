use std::env;
use std::io::IsTerminal;
use std::path::Path;
use std::process::{Command, ExitCode};
use std::time::Duration;

use reqwest::blocking::Client;
use serde_json::Value;

const LINE: &str = "------------------------------";

struct Styles {
    bold: &'static str,
    header: &'static str,
    line: &'static str,
    reset: &'static str,
}

impl Styles {
    fn detect(no_color: bool) -> Self {
        if std::io::stdout().is_terminal() && !no_color {
            Self {
                bold: "\x1b[1m",
                header: "\x1b[36m",
                line: "\x1b[90m",
                reset: "\x1b[0m",
            }
        } else {
            Self {
                bold: "",
                header: "",
                line: "",
                reset: "",
            }
        }
    }
}

#[derive(Debug)]
struct Config {
    show_public: bool,
    timeout_seconds: u64,
    no_color: bool,
}

#[derive(Debug, Clone, Default)]
struct InterfaceInfo {
    ipv4: Option<String>,
    ipv6_global: Option<String>,
    ipv6_any: Option<String>,
}

impl InterfaceInfo {
    fn display_v6(&self) -> Option<&str> {
        self.ipv6_global
            .as_deref()
            .or(self.ipv6_any.as_deref())
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(Error::Message(message, code)) => {
            eprintln!("{message}");
            ExitCode::from(code)
        }
    }
}

enum Error {
    Message(String, u8),
}

fn run() -> Result<(), Error> {
    let config = parse_args(env::args().skip(1).collect())?;

    ensure_command_exists("/sbin/ifconfig", "ifconfig")?;

    let styles = Styles::detect(config.no_color);
    let ifconfig_cmd = resolve_cmd("/sbin/ifconfig", "ifconfig");
    let route_cmd = resolve_cmd("/sbin/route", "route");

    let active_iface = determine_active_interface(&ifconfig_cmd, &route_cmd);
    let local_info = active_iface
        .as_deref()
        .and_then(|iface| load_interface_info(&ifconfig_cmd, iface));

    let public = if config.show_public {
        Some(fetch_public_info(config.timeout_seconds))
    } else {
        None
    };

    let vpn_entries = list_interfaces(&ifconfig_cmd)
        .into_iter()
        .filter(|name| is_vpn_interface(name))
        .filter_map(|name| {
            let info = load_interface_info(&ifconfig_cmd, &name)?;
            if info.ipv4.is_none() && info.ipv6_global.is_none() {
                return None;
            }
            Some((name, info))
        })
        .collect::<Vec<_>>();

    print_section(&styles, "IP locale");
    println!(
        "  {:<27} : {}",
        "Interface active",
        active_iface.as_deref().unwrap_or("indisponible")
    );
    println!(
        "  {:<27} : {}",
        "IPv4",
        local_info
            .as_ref()
            .and_then(|info| info.ipv4.as_deref())
            .unwrap_or("indisponible")
    );
    println!(
        "  {:<27} : {}",
        "IPv6",
        local_info
            .as_ref()
            .and_then(InterfaceInfo::display_v6)
            .unwrap_or("indisponible")
    );

    println!();
    print_section(&styles, "IP publique");
    if let Some(public) = public {
        println!(
            "  {:<27} : {}",
            "IPv4",
            public.ipv4.as_deref().unwrap_or("indisponible")
        );
        println!(
            "  {:<27} : {}",
            "IPv6",
            public.ipv6.as_deref().unwrap_or("indisponible")
        );
        if let Some(code) = public.country_code {
            if let Some(flag) = country_flag_emoji(&code) {
                println!("  {:<27} : {} {}", "Pays (GeoIP)", code, flag);
            } else {
                println!("  {:<27} : {}", "Pays (GeoIP)", code);
            }
        } else {
            println!("  {:<27} : indisponible", "Pays (GeoIP)");
        }
    } else {
        println!("  {:<27} : desactivee (--no-public)", "Statut");
    }

    println!();
    print_section(&styles, "VPN");
    if vpn_entries.is_empty() {
        println!("  Aucun VPN detecte");
    } else {
        for (iface, info) in vpn_entries {
            if let Some(ipv4) = info.ipv4 {
                println!("  {:<27} : {}", format!("{iface} (IPv4)"), ipv4);
            }
            if let Some(ipv6) = info.ipv6_global {
                println!("  {:<27} : {}", format!("{iface} (IPv6)"), ipv6);
            }
        }
    }

    Ok(())
}

fn parse_args(args: Vec<String>) -> Result<Config, Error> {
    let mut show_public = true;
    let mut timeout_seconds = 3_u64;
    let mut no_color = false;

    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print_usage();
                std::process::exit(0);
            }
            "--no-public" => show_public = false,
            "--no-color" => no_color = true,
            "--timeout" => {
                let value = iter.next().ok_or_else(|| {
                    Error::Message("❌ --timeout requiert une valeur".to_string(), 2)
                })?;
                timeout_seconds = value.parse::<u64>().map_err(|_| {
                    Error::Message("❌ --timeout doit etre un entier".to_string(), 2)
                })?;
            }
            other => {
                print_usage_to_stderr();
                return Err(Error::Message(format!("❌ Argument inconnu: {other}"), 2));
            }
        }
    }

    Ok(Config {
        show_public,
        timeout_seconds,
        no_color,
    })
}

fn print_usage() {
    println!(
        "\
Usage:
  mesip [--no-public] [--timeout <seconds>] [--no-color]

Description:
  Affiche IP locale, IP publique (IPv4/IPv6) et interfaces VPN detectees."
    );
}

fn print_usage_to_stderr() {
    eprintln!(
        "\
Usage:
  mesip [--no-public] [--timeout <seconds>] [--no-color]

Description:
  Affiche IP locale, IP publique (IPv4/IPv6) et interfaces VPN detectees."
    );
}

fn print_section(styles: &Styles, title: &str) {
    println!("{}{}{}", styles.line, LINE, styles.reset);
    println!("{}{}{}{}", styles.bold, styles.header, title, styles.reset);
}

fn resolve_cmd(preferred: &str, fallback: &str) -> String {
    if Path::new(preferred).is_file() {
        preferred.to_string()
    } else {
        fallback.to_string()
    }
}

fn ensure_command_exists(preferred: &str, fallback: &str) -> Result<(), Error> {
    let available = Path::new(preferred).is_file() || command_exists(fallback);
    if available {
        Ok(())
    } else {
        Err(Error::Message(
            format!("❌ Erreur : {fallback} introuvable."),
            127,
        ))
    }
}

fn command_exists(cmd: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {cmd} >/dev/null 2>&1"))
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn command_output(cmd: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(cmd).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).to_string())
}

fn determine_active_interface(ifconfig_cmd: &str, route_cmd: &str) -> Option<String> {
    if let Some(route_output) = command_output(route_cmd, &["-n", "get", "default"]) {
        for line in route_output.lines() {
            if let Some(value) = line.trim().strip_prefix("interface:") {
                let iface = value.trim();
                if !iface.is_empty() {
                    return Some(iface.to_string());
                }
            }
        }
    }

    list_interfaces(ifconfig_cmd)
        .into_iter()
        .filter(|iface| iface != "lo0")
        .find(|iface| {
            load_interface_info(ifconfig_cmd, iface)
                .and_then(|info| info.ipv4)
                .is_some()
        })
        .or_else(|| Some("en0".to_string()))
}

fn list_interfaces(ifconfig_cmd: &str) -> Vec<String> {
    command_output(ifconfig_cmd, &["-l"])
        .unwrap_or_default()
        .split_whitespace()
        .map(ToString::to_string)
        .collect()
}

fn load_interface_info(ifconfig_cmd: &str, iface: &str) -> Option<InterfaceInfo> {
    let output = command_output(ifconfig_cmd, &[iface])?;
    let mut info = InterfaceInfo::default();

    for line in output.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("inet ") {
            let ip = rest.split_whitespace().next()?.trim().to_string();
            if !ip.starts_with("127.") && info.ipv4.is_none() {
                info.ipv4 = Some(ip);
            }
        } else if let Some(rest) = trimmed.strip_prefix("inet6 ") {
            let mut ip = rest.split_whitespace().next()?.trim().to_string();
            if let Some((head, _)) = ip.split_once('%') {
                ip = head.to_string();
            }
            if info.ipv6_any.is_none() {
                info.ipv6_any = Some(ip.clone());
            }
            if !ip.starts_with("fe80:") && info.ipv6_global.is_none() {
                info.ipv6_global = Some(ip);
            }
        }
    }

    Some(info)
}

#[derive(Default)]
struct PublicInfo {
    ipv4: Option<String>,
    ipv6: Option<String>,
    country_code: Option<String>,
}

fn fetch_public_info(timeout_seconds: u64) -> PublicInfo {
    let client = Client::builder()
        .timeout(Duration::from_secs(timeout_seconds))
        .build()
        .ok();

    let ipv4 = fetch_public_ip(
        client.as_ref(),
        &[
            "https://api.ipify.org",
            "https://ifconfig.me/ip",
            "https://ipv4.icanhazip.com",
        ],
        IpFamily::V4,
    );
    let ipv6 = fetch_public_ip(
        client.as_ref(),
        &[
            "https://api64.ipify.org",
            "https://ifconfig.me/ip",
            "https://ipv6.icanhazip.com",
        ],
        IpFamily::V6,
    );

    let geo_ip = ipv4.as_deref().or(ipv6.as_deref());
    let country_code = geo_ip.and_then(|ip| fetch_country_code(client.as_ref(), ip));

    PublicInfo {
        ipv4,
        ipv6,
        country_code,
    }
}

enum IpFamily {
    V4,
    V6,
}

fn fetch_public_ip(client: Option<&Client>, urls: &[&str], family: IpFamily) -> Option<String> {
    let client = client?;
    for url in urls {
        let response = match client.get(*url).send() {
            Ok(response) => response,
            Err(_) => continue,
        };
        let text = match response.text() {
            Ok(text) => text,
            Err(_) => continue,
        };
        let line = text.lines().next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let valid = match family {
            IpFamily::V4 => is_ipv4(line),
            IpFamily::V6 => is_ipv6(line),
        };
        if valid {
            return Some(line.to_string());
        }
    }
    None
}

fn fetch_country_code(client: Option<&Client>, ip: &str) -> Option<String> {
    let client = client?;
    let urls = [
        format!("https://ipapi.co/{ip}/country/"),
        format!("https://ipinfo.io/{ip}/country"),
        format!("https://ipwho.is/{ip}"),
    ];

    for url in &urls {
        let response = match client.get(url).send() {
            Ok(response) => response,
            Err(_) => continue,
        };
        let text = match response.text() {
            Ok(text) => text,
            Err(_) => continue,
        };
        let code = if url.contains("ipwho.is") {
            parse_country_code_json(&text)
        } else {
            let line = text.lines().next().unwrap_or("").trim().to_uppercase();
            if is_country_code(&line) {
                Some(line)
            } else {
                None
            }
        };
        if code.is_some() {
            return code;
        }
    }

    None
}

fn parse_country_code_json(text: &str) -> Option<String> {
    let value: Value = serde_json::from_str(text).ok()?;
    let code = value.get("country_code")?.as_str()?.trim().to_uppercase();
    if is_country_code(&code) {
        Some(code)
    } else {
        None
    }
}

fn is_ipv4(value: &str) -> bool {
    let parts = value.split('.').collect::<Vec<_>>();
    if parts.len() != 4 {
        return false;
    }
    parts.iter().all(|part| part.parse::<u8>().is_ok())
}

fn is_ipv6(value: &str) -> bool {
    value.contains(':')
}

fn is_country_code(value: &str) -> bool {
    value.len() == 2 && value.chars().all(|c| c.is_ascii_alphabetic())
}

fn country_flag_emoji(code: &str) -> Option<String> {
    if !is_country_code(code) {
        return None;
    }

    let mut chars = code.chars().map(|c| c.to_ascii_uppercase());
    let first = chars.next()?;
    let second = chars.next()?;
    let first = char::from_u32(first as u32 + 127397)?;
    let second = char::from_u32(second as u32 + 127397)?;
    Some(format!("{first}{second}"))
}

fn is_vpn_interface(name: &str) -> bool {
    if let Some(suffix) = name.strip_prefix("utun") {
        return suffix.chars().all(|c| c.is_ascii_digit());
    }
    for prefix in ["tun", "tap", "wg", "ppp"] {
        if let Some(suffix) = name.strip_prefix(prefix) {
            return suffix.chars().all(|c| c.is_ascii_digit());
        }
    }
    if let Some(suffix) = name.strip_prefix("ipsec") {
        return suffix.is_empty() || suffix.chars().all(|c| c.is_ascii_digit());
    }
    false
}
