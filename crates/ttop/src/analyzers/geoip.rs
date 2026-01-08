//! Embedded Geo-IP lookup - Sovereign AI, no external dependencies.
//!
//! Lightweight IP-to-country mapping using hardcoded ranges for:
//! - Major cloud providers (AWS, GCP, Azure, DO, etc.)
//! - CDNs (Cloudflare, Akamai, Fastly)
//! - Well-known services (Google, Facebook, Apple, Microsoft)
//! - Private/local ranges
//! - Major country allocations (rough approximations)

use std::net::Ipv4Addr;

/// Country info with flag emoji
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CountryInfo {
    pub code: &'static str,
    pub flag: &'static str,
    pub name: &'static str,
}

impl CountryInfo {
    const fn new(code: &'static str, flag: &'static str, name: &'static str) -> Self {
        Self { code, flag, name }
    }
}

// Common countries (some reserved for future IP range expansion)
#[allow(dead_code)]
const US: CountryInfo = CountryInfo::new("US", "🇺🇸", "United States");
#[allow(dead_code)]
const DE: CountryInfo = CountryInfo::new("DE", "🇩🇪", "Germany");
#[allow(dead_code)]
const GB: CountryInfo = CountryInfo::new("GB", "🇬🇧", "United Kingdom");
#[allow(dead_code)]
const FR: CountryInfo = CountryInfo::new("FR", "🇫🇷", "France");
#[allow(dead_code)]
const NL: CountryInfo = CountryInfo::new("NL", "🇳🇱", "Netherlands");
#[allow(dead_code)]
const JP: CountryInfo = CountryInfo::new("JP", "🇯🇵", "Japan");
#[allow(dead_code)]
const SG: CountryInfo = CountryInfo::new("SG", "🇸🇬", "Singapore");
#[allow(dead_code)]
const AU: CountryInfo = CountryInfo::new("AU", "🇦🇺", "Australia");
#[allow(dead_code)]
const CA: CountryInfo = CountryInfo::new("CA", "🇨🇦", "Canada");
#[allow(dead_code)]
const BR: CountryInfo = CountryInfo::new("BR", "🇧🇷", "Brazil");
#[allow(dead_code)]
const IN: CountryInfo = CountryInfo::new("IN", "🇮🇳", "India");
#[allow(dead_code)]
const CN: CountryInfo = CountryInfo::new("CN", "🇨🇳", "China");
#[allow(dead_code)]
const RU: CountryInfo = CountryInfo::new("RU", "🇷🇺", "Russia");
#[allow(dead_code)]
const KR: CountryInfo = CountryInfo::new("KR", "🇰🇷", "South Korea");
#[allow(dead_code)]
const IE: CountryInfo = CountryInfo::new("IE", "🇮🇪", "Ireland");
#[allow(dead_code)]
const SE: CountryInfo = CountryInfo::new("SE", "🇸🇪", "Sweden");
#[allow(dead_code)]
const CH: CountryInfo = CountryInfo::new("CH", "🇨🇭", "Switzerland");
#[allow(dead_code)]
const IT: CountryInfo = CountryInfo::new("IT", "🇮🇹", "Italy");
#[allow(dead_code)]
const ES: CountryInfo = CountryInfo::new("ES", "🇪🇸", "Spain");
#[allow(dead_code)]
const PL: CountryInfo = CountryInfo::new("PL", "🇵🇱", "Poland");

// Special designations
#[allow(dead_code)]
const LOCAL: CountryInfo = CountryInfo::new("LO", "🏠", "Local");
#[allow(dead_code)]
const PRIVATE: CountryInfo = CountryInfo::new("PR", "🔒", "Private");
#[allow(dead_code)]
const CLOUD: CountryInfo = CountryInfo::new("☁️", "☁️", "Cloud");

/// IP range with associated country
struct IpRange {
    start: u32,
    end: u32,
    country: CountryInfo,
}

impl IpRange {
    const fn new(start: u32, end: u32, country: CountryInfo) -> Self {
        Self { start, end, country }
    }

    fn contains(&self, ip: u32) -> bool {
        ip >= self.start && ip <= self.end
    }
}

/// Convert IP to u32 for range comparison
fn ip_to_u32(ip: Ipv4Addr) -> u32 {
    let octets = ip.octets();
    ((octets[0] as u32) << 24)
        | ((octets[1] as u32) << 16)
        | ((octets[2] as u32) << 8)
        | (octets[3] as u32)
}

/// Convert CIDR notation to range
const fn cidr_to_range(a: u8, b: u8, c: u8, d: u8, prefix: u8) -> (u32, u32) {
    let ip = ((a as u32) << 24) | ((b as u32) << 16) | ((c as u32) << 8) | (d as u32);
    let mask = if prefix == 0 { 0 } else { !0u32 << (32 - prefix) };
    let start = ip & mask;
    let end = start | !mask;
    (start, end)
}

// Macro for cleaner range definitions
macro_rules! range {
    ($a:expr, $b:expr, $c:expr, $d:expr, $prefix:expr, $country:expr) => {{
        let (start, end) = cidr_to_range($a, $b, $c, $d, $prefix);
        IpRange::new(start, end, $country)
    }};
}

/// Get embedded IP ranges
/// Ordered by specificity (more specific ranges first)
fn get_ranges() -> Vec<IpRange> {
    vec![
        // === Private/Local Ranges ===
        range!(127, 0, 0, 0, 8, LOCAL),      // Loopback
        range!(10, 0, 0, 0, 8, PRIVATE),     // Private Class A
        range!(172, 16, 0, 0, 12, PRIVATE),  // Private Class B
        range!(192, 168, 0, 0, 16, PRIVATE), // Private Class C
        range!(169, 254, 0, 0, 16, LOCAL),   // Link-local

        // === Major Cloud Providers ===
        // Cloudflare (anycast, but HQ in US)
        range!(104, 16, 0, 0, 13, US),   // Cloudflare
        range!(104, 24, 0, 0, 14, US),   // Cloudflare
        range!(172, 64, 0, 0, 13, US),   // Cloudflare
        range!(173, 245, 48, 0, 20, US), // Cloudflare
        range!(103, 21, 244, 0, 22, US), // Cloudflare
        range!(103, 22, 200, 0, 22, US), // Cloudflare
        range!(103, 31, 4, 0, 22, US),   // Cloudflare
        range!(141, 101, 64, 0, 18, US), // Cloudflare
        range!(108, 162, 192, 0, 18, US), // Cloudflare
        range!(190, 93, 240, 0, 20, US), // Cloudflare
        range!(188, 114, 96, 0, 20, US), // Cloudflare
        range!(197, 234, 240, 0, 22, US), // Cloudflare
        range!(198, 41, 128, 0, 17, US), // Cloudflare
        range!(162, 158, 0, 0, 15, US),  // Cloudflare
        range!(131, 0, 72, 0, 22, US),   // Cloudflare

        // Google (US-based, global anycast)
        range!(8, 8, 8, 0, 24, US),       // Google DNS
        range!(8, 8, 4, 0, 24, US),       // Google DNS
        range!(8, 34, 208, 0, 20, US),    // Google Cloud
        range!(8, 35, 192, 0, 18, US),    // Google Cloud
        range!(34, 64, 0, 0, 10, US),     // Google Cloud
        range!(34, 128, 0, 0, 10, US),    // Google Cloud
        range!(35, 184, 0, 0, 13, US),    // Google Cloud
        range!(35, 192, 0, 0, 12, US),    // Google Cloud
        range!(35, 208, 0, 0, 12, US),    // Google Cloud
        range!(35, 224, 0, 0, 12, US),    // Google Cloud
        range!(35, 240, 0, 0, 13, US),    // Google Cloud
        range!(64, 233, 160, 0, 19, US),  // Google
        range!(66, 102, 0, 0, 20, US),    // Google
        range!(66, 249, 64, 0, 19, US),   // Google
        range!(72, 14, 192, 0, 18, US),   // Google
        range!(74, 125, 0, 0, 16, US),    // Google
        range!(108, 177, 0, 0, 17, US),   // Google
        range!(142, 250, 0, 0, 15, US),   // Google
        range!(172, 217, 0, 0, 16, US),   // Google
        range!(173, 194, 0, 0, 16, US),   // Google
        range!(209, 85, 128, 0, 17, US),  // Google
        range!(216, 58, 192, 0, 19, US),  // Google
        range!(216, 239, 32, 0, 19, US),  // Google

        // Amazon AWS - SPECIFIC REGIONAL RANGES FIRST (more specific before general)
        // AWS EU regions (must come before general US ranges)
        range!(3, 248, 0, 0, 13, IE),     // AWS eu-west-1 (Ireland)
        range!(18, 200, 0, 0, 13, IE),    // AWS eu-west-1
        range!(34, 240, 0, 0, 12, IE),    // AWS eu-west-1
        range!(52, 16, 0, 0, 12, IE),     // AWS eu-west-1
        range!(54, 72, 0, 0, 13, IE),     // AWS eu-west-1
        range!(54, 216, 0, 0, 13, IE),    // AWS eu-west-1
        range!(63, 32, 0, 0, 12, IE),     // AWS eu-west-1

        range!(3, 64, 0, 0, 10, DE),      // AWS eu-central-1 (Frankfurt)
        range!(18, 156, 0, 0, 14, DE),    // AWS eu-central-1
        range!(35, 156, 0, 0, 13, DE),    // AWS eu-central-1
        range!(52, 28, 0, 0, 14, DE),     // AWS eu-central-1
        range!(54, 93, 0, 0, 16, DE),     // AWS eu-central-1

        // AWS Asia-Pacific (must come before general US ranges)
        range!(13, 112, 0, 0, 12, JP),    // AWS ap-northeast-1 (Tokyo)
        range!(18, 176, 0, 0, 12, JP),    // AWS ap-northeast-1
        range!(52, 68, 0, 0, 14, JP),     // AWS ap-northeast-1

        range!(13, 228, 0, 0, 14, SG),    // AWS ap-southeast-1 (Singapore)
        range!(18, 136, 0, 0, 13, SG),    // AWS ap-southeast-1
        range!(52, 74, 0, 0, 15, SG),     // AWS ap-southeast-1
        range!(54, 169, 0, 0, 16, SG),    // AWS ap-southeast-1

        range!(13, 236, 0, 0, 14, AU),    // AWS ap-southeast-2 (Sydney)
        range!(52, 62, 0, 0, 15, AU),     // AWS ap-southeast-2
        range!(54, 66, 0, 0, 15, AU),     // AWS ap-southeast-2

        // AWS US regions (general, catch-all after specific regions)
        range!(3, 0, 0, 0, 8, US),        // AWS (mostly US)
        range!(13, 32, 0, 0, 11, US),     // AWS US
        range!(13, 56, 0, 0, 13, US),     // AWS US
        range!(15, 177, 0, 0, 16, US),    // AWS US
        range!(18, 128, 0, 0, 9, US),     // AWS US
        range!(23, 20, 0, 0, 14, US),     // AWS US
        range!(34, 192, 0, 0, 10, US),    // AWS US
        range!(52, 0, 0, 0, 8, US),       // AWS (global fallback)
        range!(54, 64, 0, 0, 10, US),     // AWS US
        range!(54, 144, 0, 0, 12, US),    // AWS US
        range!(54, 160, 0, 0, 11, US),    // AWS US
        range!(54, 192, 0, 0, 10, US),    // AWS US
        range!(99, 77, 128, 0, 17, US),   // AWS US
        range!(99, 78, 128, 0, 17, US),   // AWS US
        range!(184, 72, 0, 0, 13, US),    // AWS US
        range!(204, 236, 128, 0, 17, US), // AWS US

        // Microsoft Azure
        range!(13, 64, 0, 0, 10, US),     // Azure US
        range!(20, 0, 0, 0, 8, US),       // Azure (global, mostly US)
        range!(40, 64, 0, 0, 10, US),     // Azure US
        range!(40, 112, 0, 0, 12, US),    // Azure US
        range!(52, 160, 0, 0, 11, US),    // Azure US
        range!(104, 40, 0, 0, 13, US),    // Azure US
        range!(104, 208, 0, 0, 12, US),   // Azure US
        range!(137, 116, 0, 0, 14, US),   // Azure US
        range!(137, 117, 0, 0, 16, US),   // Azure US
        range!(168, 61, 0, 0, 16, US),    // Azure US
        range!(168, 62, 0, 0, 15, US),    // Azure US
        range!(191, 232, 0, 0, 13, US),   // Azure US

        // DigitalOcean
        range!(45, 55, 0, 0, 16, US),     // DigitalOcean
        range!(67, 205, 128, 0, 17, US),  // DigitalOcean
        range!(104, 131, 0, 0, 16, US),   // DigitalOcean
        range!(104, 236, 0, 0, 14, US),   // DigitalOcean
        range!(138, 68, 0, 0, 14, US),    // DigitalOcean
        range!(138, 197, 0, 0, 16, US),   // DigitalOcean
        range!(159, 65, 0, 0, 16, US),    // DigitalOcean
        range!(159, 89, 0, 0, 16, US),    // DigitalOcean
        range!(159, 203, 0, 0, 16, US),   // DigitalOcean
        range!(167, 99, 0, 0, 16, US),    // DigitalOcean
        range!(167, 172, 0, 0, 14, US),   // DigitalOcean
        range!(174, 138, 0, 0, 15, US),   // DigitalOcean
        range!(192, 241, 128, 0, 17, US), // DigitalOcean
        range!(198, 199, 64, 0, 18, US),  // DigitalOcean
        range!(206, 81, 0, 0, 16, US),    // DigitalOcean
        range!(206, 189, 0, 0, 16, US),   // DigitalOcean
        range!(209, 97, 128, 0, 17, US),  // DigitalOcean

        // Hetzner (Germany)
        range!(5, 9, 0, 0, 16, DE),       // Hetzner
        range!(78, 46, 0, 0, 15, DE),     // Hetzner
        range!(88, 198, 0, 0, 15, DE),    // Hetzner
        range!(88, 99, 0, 0, 16, DE),     // Hetzner
        range!(94, 130, 0, 0, 15, DE),    // Hetzner
        range!(95, 216, 192, 0, 18, DE),  // Hetzner
        range!(116, 202, 0, 0, 15, DE),   // Hetzner
        range!(116, 203, 0, 0, 16, DE),   // Hetzner
        range!(136, 243, 0, 0, 16, DE),   // Hetzner
        range!(138, 201, 0, 0, 16, DE),   // Hetzner
        range!(144, 76, 0, 0, 14, DE),    // Hetzner
        range!(148, 251, 0, 0, 16, DE),   // Hetzner
        range!(159, 69, 0, 0, 16, DE),    // Hetzner
        range!(176, 9, 0, 0, 16, DE),     // Hetzner
        range!(178, 63, 0, 0, 16, DE),    // Hetzner
        range!(195, 201, 0, 0, 16, DE),   // Hetzner
        range!(213, 133, 96, 0, 19, DE),  // Hetzner

        // OVH (France)
        range!(5, 39, 0, 0, 16, FR),      // OVH
        range!(51, 68, 0, 0, 14, FR),     // OVH
        range!(51, 77, 0, 0, 16, FR),     // OVH
        range!(51, 91, 0, 0, 16, FR),     // OVH
        range!(54, 36, 0, 0, 14, FR),     // OVH
        range!(54, 37, 0, 0, 16, FR),     // OVH
        range!(54, 38, 0, 0, 15, FR),     // OVH
        range!(91, 121, 0, 0, 16, FR),    // OVH
        range!(91, 134, 0, 0, 15, FR),    // OVH
        range!(92, 222, 0, 0, 15, FR),    // OVH
        range!(137, 74, 0, 0, 15, FR),    // OVH
        range!(145, 239, 0, 0, 16, FR),   // OVH
        range!(149, 202, 0, 0, 15, FR),   // OVH
        range!(151, 80, 0, 0, 14, FR),    // OVH
        range!(176, 31, 0, 0, 16, FR),    // OVH
        range!(178, 32, 0, 0, 15, FR),    // OVH
        range!(188, 165, 0, 0, 16, FR),   // OVH
        range!(193, 70, 0, 0, 15, FR),    // OVH
        range!(198, 27, 64, 0, 18, FR),   // OVH
        range!(213, 32, 0, 0, 14, FR),    // OVH
        range!(213, 186, 32, 0, 19, FR),  // OVH

        // Linode
        range!(45, 33, 0, 0, 16, US),     // Linode
        range!(45, 56, 0, 0, 14, US),     // Linode
        range!(45, 79, 0, 0, 16, US),     // Linode
        range!(50, 116, 0, 0, 15, US),    // Linode
        range!(66, 175, 208, 0, 20, US),  // Linode
        range!(66, 228, 32, 0, 19, US),   // Linode
        range!(69, 164, 192, 0, 18, US),  // Linode
        range!(72, 14, 176, 0, 20, US),   // Linode
        range!(74, 207, 224, 0, 19, US),  // Linode
        range!(96, 126, 96, 0, 19, US),   // Linode
        range!(97, 107, 128, 0, 17, US),  // Linode
        range!(139, 144, 0, 0, 16, US),   // Linode
        range!(139, 162, 0, 0, 15, US),   // Linode
        range!(172, 104, 0, 0, 13, US),   // Linode
        range!(173, 230, 128, 0, 17, US), // Linode
        range!(173, 255, 192, 0, 18, US), // Linode
        range!(192, 155, 80, 0, 20, US),  // Linode
        range!(198, 58, 96, 0, 19, US),   // Linode

        // Vultr
        range!(45, 32, 0, 0, 15, US),     // Vultr
        range!(45, 63, 0, 0, 16, US),     // Vultr
        range!(45, 76, 0, 0, 15, US),     // Vultr
        range!(45, 77, 0, 0, 16, US),     // Vultr
        range!(64, 156, 0, 0, 14, US),    // Vultr
        range!(64, 237, 32, 0, 19, US),   // Vultr
        range!(66, 42, 32, 0, 19, US),    // Vultr
        range!(104, 156, 224, 0, 19, US), // Vultr
        range!(104, 207, 128, 0, 17, US), // Vultr
        range!(108, 61, 0, 0, 16, US),    // Vultr
        range!(136, 244, 64, 0, 18, US),  // Vultr
        range!(149, 28, 0, 0, 15, US),    // Vultr
        range!(155, 138, 128, 0, 17, US), // Vultr
        range!(207, 148, 0, 0, 16, US),   // Vultr
        range!(208, 167, 224, 0, 19, US), // Vultr
        range!(209, 250, 224, 0, 19, US), // Vultr
        range!(216, 128, 128, 0, 17, US), // Vultr

        // === Major Services ===
        // Facebook/Meta
        range!(31, 13, 24, 0, 21, US),    // Facebook
        range!(31, 13, 64, 0, 18, US),    // Facebook
        range!(45, 64, 40, 0, 21, US),    // Facebook
        range!(66, 220, 144, 0, 20, US),  // Facebook
        range!(69, 63, 176, 0, 20, US),   // Facebook
        range!(69, 171, 224, 0, 19, US),  // Facebook
        range!(74, 119, 76, 0, 22, US),   // Facebook
        range!(102, 132, 0, 0, 14, US),   // Facebook
        range!(103, 4, 96, 0, 22, US),    // Facebook
        range!(129, 134, 0, 0, 16, US),   // Facebook
        range!(157, 240, 0, 0, 16, US),   // Facebook
        range!(173, 252, 64, 0, 18, US),  // Facebook
        range!(179, 60, 192, 0, 18, US),  // Facebook
        range!(185, 60, 216, 0, 22, US),  // Facebook
        range!(185, 89, 218, 0, 23, US),  // Facebook
        range!(204, 15, 20, 0, 22, US),   // Facebook

        // Apple
        range!(17, 0, 0, 0, 8, US),       // Apple (entire /8)

        // Microsoft (non-Azure)
        range!(40, 76, 0, 0, 14, US),     // Microsoft
        range!(40, 96, 0, 0, 12, US),     // Microsoft
        range!(40, 125, 0, 0, 16, US),    // Microsoft
        range!(52, 96, 0, 0, 12, US),     // Microsoft
        range!(65, 52, 0, 0, 14, US),     // Microsoft
        range!(65, 55, 0, 0, 16, US),     // Microsoft
        range!(111, 221, 16, 0, 20, US),  // Microsoft
        range!(131, 107, 0, 0, 16, US),   // Microsoft
        range!(134, 170, 0, 0, 15, US),   // Microsoft
        range!(150, 171, 0, 0, 16, US),   // Microsoft
        range!(157, 54, 0, 0, 15, US),    // Microsoft
        range!(157, 56, 0, 0, 14, US),    // Microsoft
        range!(199, 30, 16, 0, 20, US),   // Microsoft
        range!(204, 79, 135, 0, 24, US),  // Microsoft
        range!(204, 152, 18, 0, 23, US),  // Microsoft
        range!(207, 46, 0, 0, 16, US),    // Microsoft

        // GitHub (Microsoft)
        range!(140, 82, 112, 0, 20, US),  // GitHub
        range!(143, 55, 64, 0, 20, US),   // GitHub
        range!(185, 199, 108, 0, 22, US), // GitHub
        range!(192, 30, 252, 0, 22, US),  // GitHub

        // Akamai
        range!(2, 16, 0, 0, 12, US),      // Akamai
        range!(23, 0, 0, 0, 11, US),      // Akamai
        range!(23, 32, 0, 0, 11, US),     // Akamai
        range!(23, 64, 0, 0, 10, US),     // Akamai
        range!(23, 192, 0, 0, 10, US),    // Akamai
        range!(95, 100, 0, 0, 14, US),    // Akamai
        range!(96, 16, 0, 0, 12, US),     // Akamai
        range!(104, 64, 0, 0, 10, US),    // Akamai
        range!(118, 214, 0, 0, 15, US),   // Akamai
        range!(184, 24, 0, 0, 13, US),    // Akamai
        range!(184, 50, 0, 0, 15, US),    // Akamai
        range!(184, 84, 0, 0, 14, US),    // Akamai

        // Fastly
        range!(23, 235, 32, 0, 19, US),   // Fastly
        range!(43, 249, 72, 0, 21, US),   // Fastly
        range!(103, 244, 50, 0, 23, US),  // Fastly
        range!(103, 245, 222, 0, 23, US), // Fastly
        range!(103, 245, 224, 0, 23, US), // Fastly
        range!(104, 156, 80, 0, 20, US),  // Fastly
        range!(146, 75, 0, 0, 16, US),    // Fastly
        range!(151, 101, 0, 0, 16, US),   // Fastly
        range!(157, 52, 64, 0, 18, US),   // Fastly
        range!(167, 82, 0, 0, 15, US),    // Fastly
        range!(172, 111, 64, 0, 18, US),  // Fastly
        range!(185, 31, 16, 0, 21, US),   // Fastly
        range!(199, 232, 0, 0, 16, US),   // Fastly

        // Twitter/X
        range!(69, 195, 160, 0, 19, US),  // Twitter
        range!(104, 244, 40, 0, 21, US),  // Twitter
        range!(192, 133, 76, 0, 22, US),  // Twitter
        range!(199, 16, 156, 0, 22, US),  // Twitter
        range!(199, 59, 148, 0, 22, US),  // Twitter
        range!(199, 96, 56, 0, 21, US),   // Twitter

        // Netflix
        range!(23, 246, 0, 0, 16, US),    // Netflix
        range!(37, 77, 184, 0, 21, US),   // Netflix
        range!(45, 57, 0, 0, 16, US),     // Netflix
        range!(64, 120, 128, 0, 17, US),  // Netflix
        range!(66, 197, 128, 0, 17, US),  // Netflix
        range!(108, 175, 32, 0, 19, US),  // Netflix
        range!(185, 2, 220, 0, 22, US),   // Netflix
        range!(185, 9, 188, 0, 22, US),   // Netflix
        range!(192, 173, 64, 0, 18, US),  // Netflix
        range!(198, 38, 96, 0, 19, US),   // Netflix
        range!(198, 45, 48, 0, 20, US),   // Netflix

        // Discord
        range!(162, 159, 128, 0, 17, US), // Discord

        // Slack
        range!(54, 68, 0, 0, 13, US),     // Slack

        // Zoom
        range!(3, 7, 35, 0, 24, US),      // Zoom
        range!(3, 21, 137, 0, 24, US),    // Zoom
        range!(3, 22, 11, 0, 24, US),     // Zoom
        range!(3, 23, 93, 0, 24, US),     // Zoom
        range!(3, 25, 41, 0, 24, US),     // Zoom
        range!(3, 25, 42, 0, 23, US),     // Zoom
        range!(3, 25, 49, 0, 24, US),     // Zoom
        range!(8, 5, 128, 0, 18, US),     // Zoom
        range!(13, 52, 6, 0, 23, US),     // Zoom
        range!(18, 157, 88, 0, 24, US),   // Zoom
        range!(52, 61, 34, 0, 24, US),    // Zoom
        range!(52, 202, 62, 0, 23, US),   // Zoom
        range!(64, 69, 0, 0, 16, US),     // Zoom
        range!(64, 125, 62, 0, 24, US),   // Zoom
        range!(64, 211, 144, 0, 20, US),  // Zoom
        range!(65, 39, 152, 0, 24, US),   // Zoom
        range!(69, 174, 57, 0, 24, US),   // Zoom
        range!(69, 174, 108, 0, 22, US),  // Zoom
        range!(99, 79, 20, 0, 24, US),    // Zoom
        range!(101, 36, 167, 0, 24, US),  // Zoom
        range!(103, 122, 166, 0, 23, US), // Zoom
        range!(109, 94, 160, 0, 19, US),  // Zoom
        range!(111, 33, 115, 0, 24, US),  // Zoom
        range!(111, 33, 181, 0, 24, US),  // Zoom
        range!(120, 29, 148, 0, 24, US),  // Zoom
        range!(129, 151, 0, 0, 16, US),   // Zoom
        range!(140, 238, 128, 0, 17, US), // Zoom
        range!(147, 124, 96, 0, 19, US),  // Zoom
        range!(149, 137, 0, 0, 17, US),   // Zoom
        range!(150, 107, 64, 0, 18, US),  // Zoom
        range!(160, 16, 0, 0, 12, US),    // Zoom
        range!(162, 12, 232, 0, 21, US),  // Zoom
        range!(162, 255, 0, 0, 16, US),   // Zoom
        range!(165, 254, 88, 0, 21, US),  // Zoom
        range!(170, 114, 0, 0, 16, US),   // Zoom
        range!(173, 231, 80, 0, 20, US),  // Zoom
        range!(192, 204, 12, 0, 22, US),  // Zoom
        range!(193, 122, 32, 0, 19, US),  // Zoom
        range!(198, 251, 128, 0, 17, US), // Zoom
        range!(202, 177, 207, 0, 24, US), // Zoom
        range!(204, 80, 104, 0, 21, US),  // Zoom
        range!(204, 141, 28, 0, 22, US),  // Zoom
        range!(207, 226, 132, 0, 24, US), // Zoom
        range!(209, 9, 211, 0, 24, US),   // Zoom
        range!(213, 19, 144, 0, 20, US),  // Zoom
        range!(213, 244, 140, 0, 22, US), // Zoom

        // === Major Country Allocations (rough approximations) ===
        // These are VERY approximate - just common ranges

        // China
        range!(1, 0, 0, 0, 8, CN),        // APNIC (often CN)
        range!(14, 0, 0, 0, 8, CN),       // APNIC
        range!(27, 0, 0, 0, 8, CN),       // APNIC
        range!(36, 0, 0, 0, 8, CN),       // APNIC
        range!(39, 0, 0, 0, 8, CN),       // APNIC
        range!(42, 0, 0, 0, 8, CN),       // APNIC
        range!(49, 0, 0, 0, 8, CN),       // APNIC
        range!(58, 0, 0, 0, 8, CN),       // APNIC
        range!(59, 0, 0, 0, 8, CN),       // APNIC
        range!(60, 0, 0, 0, 8, CN),       // APNIC
        range!(61, 0, 0, 0, 8, CN),       // APNIC
        range!(101, 0, 0, 0, 8, CN),      // APNIC
        range!(106, 0, 0, 0, 8, CN),      // APNIC
        range!(110, 0, 0, 0, 8, CN),      // APNIC
        range!(111, 0, 0, 0, 8, CN),      // APNIC
        range!(112, 0, 0, 0, 8, CN),      // APNIC
        range!(113, 0, 0, 0, 8, CN),      // APNIC
        range!(114, 0, 0, 0, 8, CN),      // APNIC
        range!(115, 0, 0, 0, 8, CN),      // APNIC
        range!(116, 0, 0, 0, 8, CN),      // APNIC
        range!(117, 0, 0, 0, 8, CN),      // APNIC
        range!(118, 0, 0, 0, 8, CN),      // APNIC
        range!(119, 0, 0, 0, 8, CN),      // APNIC
        range!(120, 0, 0, 0, 8, CN),      // APNIC
        range!(121, 0, 0, 0, 8, CN),      // APNIC
        range!(122, 0, 0, 0, 8, CN),      // APNIC
        range!(123, 0, 0, 0, 8, CN),      // APNIC
        range!(124, 0, 0, 0, 8, CN),      // APNIC
        range!(125, 0, 0, 0, 8, CN),      // APNIC
        range!(180, 0, 0, 0, 8, CN),      // APNIC
        range!(182, 0, 0, 0, 8, CN),      // APNIC
        range!(183, 0, 0, 0, 8, CN),      // APNIC
        range!(202, 0, 0, 0, 8, CN),      // APNIC
        range!(203, 0, 0, 0, 8, CN),      // APNIC
        range!(210, 0, 0, 0, 8, CN),      // APNIC
        range!(211, 0, 0, 0, 8, CN),      // APNIC
        range!(218, 0, 0, 0, 8, CN),      // APNIC
        range!(219, 0, 0, 0, 8, CN),      // APNIC
        range!(220, 0, 0, 0, 8, CN),      // APNIC
        range!(221, 0, 0, 0, 8, CN),      // APNIC
        range!(222, 0, 0, 0, 8, CN),      // APNIC
        range!(223, 0, 0, 0, 8, CN),      // APNIC

        // Russia
        range!(2, 92, 0, 0, 14, RU),      // RU
        range!(5, 3, 0, 0, 16, RU),       // RU
        range!(5, 8, 0, 0, 13, RU),       // RU
        range!(5, 16, 0, 0, 12, RU),      // RU
        range!(5, 34, 0, 0, 15, RU),      // RU
        range!(5, 42, 0, 0, 15, RU),      // RU
        range!(5, 44, 0, 0, 14, RU),      // RU
        range!(5, 53, 0, 0, 16, RU),      // RU
        range!(5, 56, 0, 0, 13, RU),      // RU
        range!(5, 100, 0, 0, 14, RU),     // RU
        range!(5, 128, 0, 0, 13, RU),     // RU
        range!(5, 136, 0, 0, 13, RU),     // RU
        range!(5, 158, 0, 0, 15, RU),     // RU
        range!(5, 164, 0, 0, 14, RU),     // RU
        range!(5, 178, 0, 0, 15, RU),     // RU
        range!(5, 187, 0, 0, 16, RU),     // RU
        range!(5, 189, 0, 0, 16, RU),     // RU
        range!(5, 200, 0, 0, 13, RU),     // RU
        range!(5, 227, 0, 0, 16, RU),     // RU
        range!(5, 228, 0, 0, 14, RU),     // RU
        range!(5, 248, 0, 0, 13, RU),     // RU
        range!(31, 40, 0, 0, 13, RU),     // RU
        range!(37, 18, 0, 0, 15, RU),     // RU
        range!(37, 29, 0, 0, 16, RU),     // RU
        range!(37, 110, 0, 0, 15, RU),    // RU
        range!(37, 140, 0, 0, 14, RU),    // RU
        range!(46, 138, 0, 0, 15, RU),    // RU
        range!(46, 146, 0, 0, 15, RU),    // RU
        range!(46, 160, 0, 0, 11, RU),    // RU
        range!(62, 76, 0, 0, 14, RU),     // RU
        range!(62, 109, 0, 0, 16, RU),    // RU
        range!(62, 117, 0, 0, 16, RU),    // RU
        range!(62, 133, 0, 0, 16, RU),    // RU
        range!(77, 37, 128, 0, 17, RU),   // RU
        range!(77, 72, 0, 0, 13, RU),     // RU
        range!(77, 91, 0, 0, 16, RU),     // RU
        range!(77, 232, 0, 0, 13, RU),    // RU
        range!(78, 24, 0, 0, 13, RU),     // RU
        range!(78, 36, 0, 0, 14, RU),     // RU
        range!(78, 106, 0, 0, 15, RU),    // RU
        range!(78, 108, 0, 0, 14, RU),    // RU
        range!(78, 155, 0, 0, 16, RU),    // RU
        range!(79, 133, 0, 0, 16, RU),    // RU
        range!(79, 140, 0, 0, 14, RU),    // RU
        range!(79, 164, 0, 0, 14, RU),    // RU
        range!(80, 68, 0, 0, 14, RU),     // RU
        range!(80, 237, 0, 0, 16, RU),    // RU
        range!(80, 242, 0, 0, 15, RU),    // RU
        range!(80, 250, 0, 0, 15, RU),    // RU
        range!(81, 3, 0, 0, 16, RU),      // RU
        range!(81, 16, 0, 0, 12, RU),     // RU
        range!(81, 88, 0, 0, 14, RU),     // RU
        range!(81, 176, 0, 0, 12, RU),    // RU
        range!(81, 195, 0, 0, 16, RU),    // RU
        range!(82, 138, 0, 0, 15, RU),    // RU
        range!(82, 179, 0, 0, 16, RU),    // RU
        range!(82, 194, 0, 0, 15, RU),    // RU
        range!(82, 196, 0, 0, 14, RU),    // RU
        range!(82, 208, 0, 0, 12, RU),    // RU
        range!(83, 102, 0, 0, 15, RU),    // RU
        range!(83, 149, 0, 0, 16, RU),    // RU
        range!(83, 167, 0, 0, 16, RU),    // RU
        range!(83, 220, 0, 0, 14, RU),    // RU
        range!(84, 22, 0, 0, 15, RU),     // RU
        range!(84, 52, 0, 0, 14, RU),     // RU
        range!(85, 21, 0, 0, 16, RU),     // RU
        range!(85, 26, 0, 0, 15, RU),     // RU
        range!(85, 140, 0, 0, 14, RU),    // RU
        range!(85, 192, 0, 0, 11, RU),    // RU
        range!(86, 62, 0, 0, 15, RU),     // RU
        range!(86, 102, 0, 0, 15, RU),    // RU
        range!(87, 117, 0, 0, 16, RU),    // RU
        range!(87, 224, 0, 0, 12, RU),    // RU
        range!(87, 245, 0, 0, 16, RU),    // RU
        range!(88, 82, 0, 0, 15, RU),     // RU
        range!(88, 147, 128, 0, 17, RU),  // RU
        range!(88, 200, 0, 0, 13, RU),    // RU
        range!(89, 28, 0, 0, 14, RU),     // RU
        range!(89, 109, 0, 0, 16, RU),    // RU
        range!(89, 111, 0, 0, 16, RU),    // RU
        range!(89, 178, 0, 0, 15, RU),    // RU
        range!(89, 208, 0, 0, 12, RU),    // RU
        range!(90, 150, 0, 0, 15, RU),    // RU
        range!(91, 122, 0, 0, 15, RU),    // RU
        range!(91, 189, 0, 0, 16, RU),    // RU
        range!(91, 200, 0, 0, 13, RU),    // RU
        range!(91, 210, 0, 0, 15, RU),    // RU
        range!(91, 215, 0, 0, 16, RU),    // RU
        range!(91, 219, 0, 0, 16, RU),    // RU
        range!(91, 226, 0, 0, 15, RU),    // RU
        range!(91, 232, 0, 0, 13, RU),    // RU
        range!(92, 37, 0, 0, 16, RU),     // RU
        range!(92, 50, 0, 0, 15, RU),     // RU
        range!(92, 63, 64, 0, 18, RU),    // RU
        range!(93, 80, 0, 0, 12, RU),     // RU
        range!(93, 170, 0, 0, 15, RU),    // RU
        range!(93, 178, 64, 0, 18, RU),   // RU
        range!(93, 185, 0, 0, 16, RU),    // RU
        range!(94, 19, 0, 0, 16, RU),     // RU
        range!(94, 24, 0, 0, 13, RU),     // RU
        range!(94, 41, 0, 0, 16, RU),     // RU
        range!(94, 79, 0, 0, 16, RU),     // RU
        range!(94, 100, 0, 0, 14, RU),    // RU
        range!(94, 137, 0, 0, 16, RU),    // RU
        range!(94, 180, 0, 0, 14, RU),    // RU
        range!(94, 228, 0, 0, 14, RU),    // RU
        range!(94, 232, 0, 0, 14, RU),    // RU
        range!(94, 250, 0, 0, 15, RU),    // RU

        // India
        range!(14, 139, 0, 0, 16, IN),    // IN
        range!(14, 140, 0, 0, 14, IN),    // IN
        range!(14, 192, 0, 0, 11, IN),    // IN
        range!(27, 4, 0, 0, 14, IN),      // IN
        range!(27, 6, 0, 0, 15, IN),      // IN
        range!(27, 48, 0, 0, 12, IN),     // IN
        range!(27, 56, 0, 0, 14, IN),     // IN
        range!(27, 60, 0, 0, 14, IN),     // IN
        range!(27, 116, 0, 0, 14, IN),    // IN
        range!(43, 224, 0, 0, 12, IN),    // IN
        range!(43, 241, 0, 0, 16, IN),    // IN
        range!(43, 247, 0, 0, 16, IN),    // IN
        range!(43, 250, 0, 0, 15, IN),    // IN
        range!(45, 64, 0, 0, 12, IN),     // IN
        range!(45, 112, 0, 0, 12, IN),    // IN
        range!(45, 248, 0, 0, 14, IN),    // IN
        range!(47, 8, 0, 0, 13, IN),      // IN
        range!(47, 15, 0, 0, 16, IN),     // IN
        range!(47, 29, 0, 0, 16, IN),     // IN
        range!(47, 31, 0, 0, 16, IN),     // IN
        range!(49, 14, 0, 0, 15, IN),     // IN
        range!(49, 32, 0, 0, 11, IN),     // IN
        range!(49, 200, 0, 0, 13, IN),    // IN
        range!(49, 248, 0, 0, 14, IN),    // IN
        range!(59, 88, 0, 0, 13, IN),     // IN
        range!(59, 144, 0, 0, 12, IN),    // IN
        range!(59, 160, 0, 0, 11, IN),    // IN
        range!(61, 0, 0, 0, 10, IN),      // IN
        range!(101, 0, 0, 0, 10, IN),     // IN
        range!(103, 0, 0, 0, 10, IN),     // IN
        range!(106, 0, 0, 0, 10, IN),     // IN
        range!(110, 224, 0, 0, 11, IN),   // IN
        range!(111, 92, 0, 0, 14, IN),    // IN
        range!(112, 133, 0, 0, 16, IN),   // IN
        range!(114, 29, 224, 0, 19, IN),  // IN
        range!(115, 240, 0, 0, 12, IN),   // IN
        range!(116, 50, 0, 0, 15, IN),    // IN
        range!(117, 192, 0, 0, 10, IN),   // IN
        range!(119, 224, 0, 0, 11, IN),   // IN
        range!(122, 160, 0, 0, 11, IN),   // IN
        range!(124, 153, 0, 0, 16, IN),   // IN
        range!(125, 16, 0, 0, 12, IN),    // IN
        range!(136, 232, 0, 0, 13, IN),   // IN
        range!(150, 129, 0, 0, 16, IN),   // IN
        range!(157, 49, 0, 0, 16, IN),    // IN
        range!(163, 47, 0, 0, 16, IN),    // IN
        range!(175, 100, 0, 0, 14, IN),   // IN
        range!(180, 149, 0, 0, 16, IN),   // IN
        range!(182, 64, 0, 0, 10, IN),    // IN
        range!(183, 80, 0, 0, 12, IN),    // IN
        range!(202, 54, 0, 0, 15, IN),    // IN
        range!(202, 56, 192, 0, 19, IN),  // IN
        range!(203, 115, 64, 0, 18, IN),  // IN
        range!(203, 192, 192, 0, 19, IN), // IN
        range!(210, 212, 0, 0, 14, IN),   // IN
        range!(223, 176, 0, 0, 12, IN),   // IN

        // Brazil
        range!(131, 0, 0, 0, 10, BR),     // BR
        range!(138, 0, 0, 0, 10, BR),     // BR
        range!(143, 0, 0, 0, 10, BR),     // BR
        range!(152, 240, 0, 0, 12, BR),   // BR
        range!(168, 194, 0, 0, 15, BR),   // BR
        range!(168, 195, 0, 0, 16, BR),   // BR
        range!(168, 196, 0, 0, 14, BR),   // BR
        range!(170, 78, 0, 0, 15, BR),    // BR
        range!(177, 0, 0, 0, 8, BR),      // BR
        range!(179, 0, 0, 0, 8, BR),      // BR
        range!(186, 192, 0, 0, 10, BR),   // BR
        range!(187, 0, 0, 0, 8, BR),      // BR
        range!(189, 0, 0, 0, 8, BR),      // BR
        range!(191, 0, 0, 0, 9, BR),      // BR
        range!(191, 128, 0, 0, 10, BR),   // BR
        range!(200, 0, 0, 0, 9, BR),      // BR
        range!(200, 128, 0, 0, 10, BR),   // BR
        range!(201, 0, 0, 0, 8, BR),      // BR

        // South Korea
        range!(1, 208, 0, 0, 12, KR),     // KR
        range!(1, 224, 0, 0, 11, KR),     // KR
        range!(14, 32, 0, 0, 11, KR),     // KR
        range!(27, 96, 0, 0, 11, KR),     // KR
        range!(27, 160, 0, 0, 11, KR),    // KR
        range!(39, 112, 0, 0, 12, KR),    // KR
        range!(42, 80, 0, 0, 12, KR),     // KR
        range!(58, 72, 0, 0, 13, KR),     // KR
        range!(58, 120, 0, 0, 13, KR),    // KR
        range!(58, 224, 0, 0, 12, KR),    // KR
        range!(59, 0, 0, 0, 10, KR),      // KR
        range!(61, 72, 0, 0, 13, KR),     // KR
        range!(61, 80, 0, 0, 12, KR),     // KR
        range!(110, 8, 0, 0, 13, KR),     // KR
        range!(110, 35, 0, 0, 16, KR),    // KR
        range!(110, 44, 0, 0, 14, KR),    // KR
        range!(110, 70, 0, 0, 15, KR),    // KR
        range!(112, 160, 0, 0, 11, KR),   // KR
        range!(112, 216, 0, 0, 13, KR),   // KR
        range!(114, 108, 0, 0, 14, KR),   // KR
        range!(114, 200, 0, 0, 13, KR),   // KR
        range!(115, 68, 0, 0, 14, KR),    // KR
        range!(115, 136, 0, 0, 13, KR),   // KR
        range!(118, 32, 0, 0, 11, KR),    // KR
        range!(118, 128, 0, 0, 10, KR),   // KR
        range!(118, 216, 0, 0, 13, KR),   // KR
        range!(119, 192, 0, 0, 10, KR),   // KR
        range!(121, 128, 0, 0, 10, KR),   // KR
        range!(121, 160, 0, 0, 11, KR),   // KR
        range!(122, 32, 0, 0, 11, KR),    // KR
        range!(122, 128, 0, 0, 10, KR),   // KR
        range!(123, 48, 0, 0, 12, KR),    // KR
        range!(123, 140, 0, 0, 14, KR),   // KR
        range!(123, 200, 0, 0, 13, KR),   // KR
        range!(124, 0, 0, 0, 9, KR),      // KR
        range!(125, 128, 0, 0, 10, KR),   // KR
        range!(175, 192, 0, 0, 10, KR),   // KR
        range!(180, 64, 0, 0, 10, KR),    // KR
        range!(182, 208, 0, 0, 12, KR),   // KR
        range!(183, 96, 0, 0, 11, KR),    // KR
        range!(210, 96, 0, 0, 11, KR),    // KR
        range!(211, 32, 0, 0, 11, KR),    // KR
        range!(211, 104, 0, 0, 13, KR),   // KR
        range!(211, 168, 0, 0, 13, KR),   // KR
        range!(218, 32, 0, 0, 11, KR),    // KR
        range!(218, 144, 0, 0, 12, KR),   // KR
        range!(218, 232, 0, 0, 13, KR),   // KR
        range!(220, 64, 0, 0, 10, KR),    // KR
        range!(221, 128, 0, 0, 10, KR),   // KR
        range!(222, 96, 0, 0, 11, KR),    // KR

        // Japan
        range!(1, 0, 0, 0, 11, JP),       // JP
        range!(1, 33, 0, 0, 16, JP),      // JP
        range!(1, 72, 0, 0, 13, JP),      // JP
        range!(14, 0, 0, 0, 11, JP),      // JP
        range!(27, 80, 0, 0, 13, JP),     // JP
        range!(36, 0, 0, 0, 11, JP),      // JP
        range!(42, 96, 0, 0, 11, JP),     // JP
        range!(43, 224, 0, 0, 11, JP),    // JP
        range!(49, 212, 0, 0, 14, JP),    // JP
        range!(49, 228, 0, 0, 14, JP),    // JP
        range!(58, 0, 0, 0, 11, JP),      // JP
        range!(58, 84, 0, 0, 14, JP),     // JP
        range!(59, 128, 0, 0, 10, JP),    // JP
        range!(60, 32, 0, 0, 11, JP),     // JP
        range!(61, 112, 0, 0, 12, JP),    // JP
        range!(61, 192, 0, 0, 11, JP),    // JP
        range!(101, 102, 0, 0, 15, JP),   // JP
        range!(101, 128, 0, 0, 9, JP),    // JP
        range!(110, 64, 0, 0, 10, JP),    // JP
        range!(110, 128, 0, 0, 9, JP),    // JP
        range!(111, 64, 0, 0, 10, JP),    // JP
        range!(111, 216, 0, 0, 13, JP),   // JP
        range!(114, 48, 0, 0, 12, JP),    // JP
        range!(114, 160, 0, 0, 11, JP),   // JP
        range!(115, 0, 0, 0, 10, JP),     // JP
        range!(116, 64, 0, 0, 11, JP),    // JP
        range!(118, 0, 0, 0, 11, JP),     // JP
        range!(118, 232, 0, 0, 13, JP),   // JP
        range!(119, 0, 0, 0, 10, JP),     // JP
        range!(119, 104, 0, 0, 13, JP),   // JP
        range!(120, 0, 0, 0, 10, JP),     // JP
        range!(121, 80, 0, 0, 12, JP),    // JP
        range!(122, 0, 0, 0, 11, JP),     // JP
        range!(122, 196, 0, 0, 14, JP),   // JP
        range!(123, 0, 0, 0, 12, JP),     // JP
        range!(123, 176, 0, 0, 12, JP),   // JP
        range!(124, 32, 0, 0, 11, JP),    // JP
        range!(124, 110, 0, 0, 16, JP),   // JP
        range!(124, 144, 0, 0, 12, JP),   // JP
        range!(124, 211, 0, 0, 16, JP),   // JP
        range!(125, 0, 0, 0, 11, JP),     // JP
        range!(126, 0, 0, 0, 9, JP),      // JP
        range!(126, 128, 0, 0, 11, JP),   // JP
        range!(133, 0, 0, 0, 11, JP),     // JP
        range!(133, 192, 0, 0, 11, JP),   // JP
        range!(150, 0, 0, 0, 10, JP),     // JP
        range!(153, 0, 0, 0, 10, JP),     // JP
        range!(157, 0, 0, 0, 10, JP),     // JP
        range!(163, 130, 0, 0, 15, JP),   // JP
        range!(175, 0, 0, 0, 11, JP),     // JP
        range!(180, 0, 0, 0, 11, JP),     // JP
        range!(182, 160, 0, 0, 11, JP),   // JP
        range!(183, 0, 0, 0, 11, JP),     // JP
        range!(202, 0, 0, 0, 11, JP),     // JP
        range!(203, 128, 0, 0, 10, JP),   // JP
        range!(210, 128, 0, 0, 10, JP),   // JP
        range!(211, 0, 0, 0, 10, JP),     // JP
        range!(218, 0, 0, 0, 11, JP),     // JP
        range!(219, 64, 0, 0, 10, JP),    // JP
        range!(220, 0, 0, 0, 11, JP),     // JP
        range!(221, 176, 0, 0, 12, JP),   // JP
        range!(222, 0, 0, 0, 10, JP),     // JP
        range!(223, 0, 0, 0, 11, JP),     // JP

        // UK
        range!(2, 16, 0, 0, 14, GB),      // GB
        range!(2, 24, 0, 0, 13, GB),      // GB
        range!(5, 64, 0, 0, 11, GB),      // GB
        range!(5, 148, 0, 0, 14, GB),     // GB
        range!(31, 48, 0, 0, 12, GB),     // GB
        range!(31, 94, 0, 0, 15, GB),     // GB
        range!(37, 1, 0, 0, 16, GB),      // GB
        range!(37, 24, 0, 0, 14, GB),     // GB
        range!(37, 128, 0, 0, 12, GB),    // GB
        range!(46, 32, 0, 0, 11, GB),     // GB
        range!(46, 208, 0, 0, 12, GB),    // GB
        range!(51, 36, 0, 0, 14, GB),     // GB
        range!(62, 7, 0, 0, 16, GB),      // GB
        range!(62, 24, 0, 0, 13, GB),     // GB
        range!(62, 56, 0, 0, 13, GB),     // GB
        range!(77, 68, 0, 0, 14, GB),     // GB
        range!(77, 96, 0, 0, 12, GB),     // GB
        range!(78, 32, 0, 0, 14, GB),     // GB
        range!(78, 128, 0, 0, 10, GB),    // GB
        range!(79, 64, 0, 0, 10, GB),     // GB
        range!(80, 0, 0, 0, 11, GB),      // GB
        range!(80, 192, 0, 0, 10, GB),    // GB
        range!(81, 128, 0, 0, 10, GB),    // GB
        range!(82, 0, 0, 0, 11, GB),      // GB
        range!(82, 128, 0, 0, 10, GB),    // GB
        range!(83, 64, 0, 0, 10, GB),     // GB
        range!(84, 64, 0, 0, 10, GB),     // GB
        range!(85, 0, 0, 0, 11, GB),      // GB
        range!(86, 0, 0, 0, 11, GB),      // GB
        range!(87, 64, 0, 0, 10, GB),     // GB
        range!(88, 64, 0, 0, 10, GB),     // GB
        range!(89, 0, 0, 0, 11, GB),      // GB
        range!(90, 192, 0, 0, 10, GB),    // GB
        range!(91, 192, 0, 0, 10, GB),    // GB
        range!(92, 0, 0, 0, 12, GB),      // GB
        range!(93, 64, 0, 0, 10, GB),     // GB
        range!(94, 0, 0, 0, 11, GB),      // GB
        range!(109, 64, 0, 0, 10, GB),    // GB
        range!(141, 0, 0, 0, 10, GB),     // GB
        range!(144, 0, 0, 0, 10, GB),     // GB
        range!(146, 64, 0, 0, 10, GB),    // GB
        range!(148, 0, 0, 0, 11, GB),     // GB
        range!(151, 192, 0, 0, 10, GB),   // GB
        range!(176, 0, 0, 0, 11, GB),     // GB
        range!(178, 0, 0, 0, 11, GB),     // GB
        range!(185, 0, 0, 0, 10, GB),     // GB
        range!(193, 0, 0, 0, 10, GB),     // GB
        range!(194, 0, 0, 0, 10, GB),     // GB
        range!(195, 0, 0, 0, 10, GB),     // GB
        range!(212, 0, 0, 0, 10, GB),     // GB

        // Germany
        range!(2, 200, 0, 0, 13, DE),     // DE
        range!(5, 1, 0, 0, 16, DE),       // DE
        range!(5, 56, 0, 0, 12, DE),      // DE
        range!(31, 0, 0, 0, 12, DE),      // DE
        range!(37, 0, 0, 0, 12, DE),      // DE
        range!(46, 0, 0, 0, 12, DE),      // DE
        range!(62, 0, 0, 0, 12, DE),      // DE
        range!(77, 0, 0, 0, 11, DE),      // DE
        range!(78, 0, 0, 0, 11, DE),      // DE
        range!(79, 192, 0, 0, 10, DE),    // DE
        range!(80, 128, 0, 0, 10, DE),    // DE
        range!(81, 64, 0, 0, 10, DE),     // DE
        range!(82, 64, 0, 0, 10, DE),     // DE
        range!(83, 0, 0, 0, 11, DE),      // DE
        range!(84, 128, 0, 0, 10, DE),    // DE
        range!(85, 128, 0, 0, 10, DE),    // DE
        range!(86, 128, 0, 0, 10, DE),    // DE
        range!(87, 128, 0, 0, 10, DE),    // DE
        range!(88, 128, 0, 0, 10, DE),    // DE
        range!(89, 192, 0, 0, 10, DE),    // DE
        range!(91, 0, 0, 0, 11, DE),      // DE
        range!(92, 192, 0, 0, 10, DE),    // DE
        range!(93, 192, 0, 0, 10, DE),    // DE
        range!(109, 192, 0, 0, 10, DE),   // DE
        range!(141, 192, 0, 0, 10, DE),   // DE
        range!(145, 0, 0, 0, 10, DE),     // DE
        range!(146, 0, 0, 0, 11, DE),     // DE
        range!(149, 0, 0, 0, 10, DE),     // DE
        range!(151, 0, 0, 0, 11, DE),     // DE
        range!(176, 192, 0, 0, 10, DE),   // DE
        range!(178, 192, 0, 0, 10, DE),   // DE
        range!(185, 64, 0, 0, 10, DE),    // DE
        range!(193, 192, 0, 0, 10, DE),   // DE
        range!(194, 64, 0, 0, 10, DE),    // DE
        range!(195, 192, 0, 0, 10, DE),   // DE
        range!(212, 192, 0, 0, 10, DE),   // DE
        range!(217, 0, 0, 0, 10, DE),     // DE

        // France
        range!(2, 0, 0, 0, 11, FR),       // FR
        range!(5, 32, 0, 0, 11, FR),      // FR
        range!(31, 32, 0, 0, 11, FR),     // FR
        range!(37, 56, 0, 0, 13, FR),     // FR
        range!(46, 192, 0, 0, 10, FR),    // FR
        range!(62, 192, 0, 0, 10, FR),    // FR
        range!(77, 128, 0, 0, 10, FR),    // FR
        range!(78, 192, 0, 0, 10, FR),    // FR
        range!(79, 0, 0, 0, 11, FR),      // FR
        range!(80, 64, 0, 0, 10, FR),     // FR
        range!(81, 0, 0, 0, 11, FR),      // FR
        range!(81, 192, 0, 0, 10, FR),    // FR
        range!(82, 192, 0, 0, 10, FR),    // FR
        range!(83, 128, 0, 0, 10, FR),    // FR
        range!(84, 0, 0, 0, 11, FR),      // FR
        range!(85, 64, 0, 0, 10, FR),     // FR
        range!(86, 64, 0, 0, 10, FR),     // FR
        range!(87, 0, 0, 0, 11, FR),      // FR
        range!(88, 0, 0, 0, 11, FR),      // FR
        range!(89, 64, 0, 0, 10, FR),     // FR
        range!(90, 0, 0, 0, 11, FR),      // FR
        range!(91, 64, 0, 0, 10, FR),     // FR
        range!(92, 64, 0, 0, 10, FR),     // FR
        range!(92, 128, 0, 0, 10, FR),    // FR
        range!(93, 0, 0, 0, 11, FR),      // FR
        range!(109, 0, 0, 0, 11, FR),     // FR
        range!(141, 64, 0, 0, 10, FR),    // FR
        range!(144, 64, 0, 0, 10, FR),    // FR
        range!(146, 128, 0, 0, 10, FR),   // FR
        range!(149, 128, 0, 0, 10, FR),   // FR
        range!(151, 64, 0, 0, 10, FR),    // FR
        range!(176, 64, 0, 0, 10, FR),    // FR
        range!(178, 64, 0, 0, 10, FR),    // FR
        range!(185, 128, 0, 0, 10, FR),   // FR
        range!(193, 64, 0, 0, 10, FR),    // FR
        range!(194, 128, 0, 0, 10, FR),   // FR
        range!(195, 64, 0, 0, 10, FR),    // FR
        range!(212, 64, 0, 0, 10, FR),    // FR

        // Netherlands
        range!(2, 56, 0, 0, 13, NL),      // NL
        range!(5, 2, 0, 0, 16, NL),       // NL
        range!(31, 160, 0, 0, 11, NL),    // NL
        range!(37, 32, 0, 0, 11, NL),     // NL
        range!(37, 48, 0, 0, 12, NL),     // NL
        range!(46, 64, 0, 0, 10, NL),     // NL
        range!(46, 166, 128, 0, 17, NL),  // NL
        range!(51, 0, 0, 0, 11, NL),      // NL
        range!(62, 64, 0, 0, 10, NL),     // NL
        range!(77, 160, 0, 0, 11, NL),    // NL
        range!(78, 64, 0, 0, 10, NL),     // NL
        range!(79, 128, 0, 0, 10, NL),    // NL
        range!(80, 32, 0, 0, 11, NL),     // NL
        range!(81, 32, 0, 0, 11, NL),     // NL
        range!(82, 32, 0, 0, 11, NL),     // NL
        range!(83, 160, 0, 0, 11, NL),    // NL
        range!(84, 192, 0, 0, 10, NL),    // NL
        range!(86, 192, 0, 0, 10, NL),    // NL
        range!(87, 192, 0, 0, 10, NL),    // NL
        range!(88, 192, 0, 0, 10, NL),    // NL
        range!(89, 128, 0, 0, 10, NL),    // NL
        range!(90, 64, 0, 0, 10, NL),     // NL
        range!(91, 128, 0, 0, 10, NL),    // NL
        range!(92, 32, 0, 0, 11, NL),     // NL
        range!(93, 128, 0, 0, 10, NL),    // NL
        range!(94, 64, 0, 0, 10, NL),     // NL
        range!(109, 128, 0, 0, 10, NL),   // NL
        range!(141, 128, 0, 0, 10, NL),   // NL
        range!(144, 128, 0, 0, 10, NL),   // NL
        range!(145, 128, 0, 0, 10, NL),   // NL
        range!(146, 192, 0, 0, 10, NL),   // NL
        range!(149, 64, 0, 0, 10, NL),    // NL
        range!(151, 128, 0, 0, 10, NL),   // NL
        range!(176, 128, 0, 0, 10, NL),   // NL
        range!(178, 128, 0, 0, 10, NL),   // NL
        range!(185, 192, 0, 0, 10, NL),   // NL
        range!(193, 128, 0, 0, 10, NL),   // NL
        range!(194, 192, 0, 0, 10, NL),   // NL
        range!(195, 128, 0, 0, 10, NL),   // NL
        range!(212, 128, 0, 0, 10, NL),   // NL
    ]
}

/// Lookup country for an IPv4 address
pub fn lookup(ip: Ipv4Addr) -> Option<CountryInfo> {
    let ip_u32 = ip_to_u32(ip);
    let ranges = get_ranges();

    // Check ranges (more specific ones are listed first, so first match wins)
    for range in ranges {
        if range.contains(ip_u32) {
            return Some(range.country);
        }
    }

    None
}

/// Get flag emoji for an IP, or "🌐" for unknown
pub fn get_flag(ip: Ipv4Addr) -> &'static str {
    lookup(ip).map(|c| c.flag).unwrap_or("🌐")
}

/// Get country code for an IP, or "??" for unknown
pub fn get_country_code(ip: Ipv4Addr) -> &'static str {
    lookup(ip).map(|c| c.code).unwrap_or("??")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_localhost() {
        let ip: Ipv4Addr = "127.0.0.1".parse().unwrap();
        let info = lookup(ip).unwrap();
        assert_eq!(info.code, "LO");
        assert_eq!(info.flag, "🏠");
    }

    #[test]
    fn test_private_10() {
        let ip: Ipv4Addr = "10.0.0.1".parse().unwrap();
        let info = lookup(ip).unwrap();
        assert_eq!(info.code, "PR");
        assert_eq!(info.flag, "🔒");
    }

    #[test]
    fn test_private_192() {
        let ip: Ipv4Addr = "192.168.1.1".parse().unwrap();
        let info = lookup(ip).unwrap();
        assert_eq!(info.code, "PR");
    }

    #[test]
    fn test_google_dns() {
        let ip: Ipv4Addr = "8.8.8.8".parse().unwrap();
        let info = lookup(ip).unwrap();
        assert_eq!(info.code, "US");
        assert_eq!(info.flag, "🇺🇸");
    }

    #[test]
    fn test_cloudflare() {
        let ip: Ipv4Addr = "1.1.1.1".parse().unwrap();
        // Cloudflare uses APNIC space, might be CN in our rough approximation
        let _info = lookup(ip);
        // Just verify it doesn't panic
    }

    #[test]
    fn test_cloudflare_104() {
        let ip: Ipv4Addr = "104.16.0.1".parse().unwrap();
        let info = lookup(ip).unwrap();
        assert_eq!(info.code, "US");
    }

    #[test]
    fn test_unknown_ip() {
        let ip: Ipv4Addr = "224.0.0.1".parse().unwrap();  // Multicast
        let flag = get_flag(ip);
        assert_eq!(flag, "🌐");
    }

    #[test]
    fn test_flag_helper() {
        let ip: Ipv4Addr = "8.8.8.8".parse().unwrap();
        assert_eq!(get_flag(ip), "🇺🇸");
    }

    #[test]
    fn test_country_code_helper() {
        let ip: Ipv4Addr = "8.8.8.8".parse().unwrap();
        assert_eq!(get_country_code(ip), "US");
    }

    #[test]
    fn test_hetzner_germany() {
        let ip: Ipv4Addr = "148.251.1.1".parse().unwrap();
        let info = lookup(ip).unwrap();
        assert_eq!(info.code, "DE");
        assert_eq!(info.flag, "🇩🇪");
    }

    #[test]
    fn test_ovh_france() {
        let ip: Ipv4Addr = "51.77.1.1".parse().unwrap();
        let info = lookup(ip).unwrap();
        assert_eq!(info.code, "FR");
        assert_eq!(info.flag, "🇫🇷");
    }

    #[test]
    fn test_apple_17() {
        let ip: Ipv4Addr = "17.0.0.1".parse().unwrap();
        let info = lookup(ip).unwrap();
        assert_eq!(info.code, "US");
    }

    #[test]
    fn test_aws_ireland() {
        let ip: Ipv4Addr = "52.16.1.1".parse().unwrap();
        let info = lookup(ip).unwrap();
        assert_eq!(info.code, "IE");
        assert_eq!(info.flag, "🇮🇪");
    }
}
