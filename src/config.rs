use std::collections::HashMap;

use once_cell::sync::Lazy;
use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub proxy: ProxyConfig,
    #[serde(default)]
    pub rules: HashMap<String, Rule>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ProxyConfig {
    pub main: Option<String>,
    pub fallback: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct Rule {
    #[serde(rename = "override")]
    pub override_action: Option<String>,
    pub proxy: Option<String>, // "on", "off"
    pub fallback: Option<String>,
}

pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    let content = std::fs::read_to_string("drover.toml").unwrap_or_default();
    toml::from_str(&content).unwrap_or_default()
});

fn format_pac_proxy(addr: &str) -> String {
    let addr = addr.trim();
    if addr.eq_ignore_ascii_case("DIRECT") {
        return "DIRECT".to_string();
    }

    // If it's already a PAC format string
    if addr.starts_with("PROXY ") || addr.starts_with("SOCKS") {
        return addr.to_string();
    }

    if let Some(stripped) = addr.strip_prefix("socks5://") {
        format!("SOCKS5 {}", stripped)
    } else if let Some(stripped) = addr.strip_prefix("socks4://") {
        format!("SOCKS {}", stripped)
    } else if let Some(stripped) = addr.strip_prefix("http://") {
        format!("PROXY {}", stripped)
    } else {
        format!("PROXY {}", addr)
    }
}

pub fn generate_pac_script(config: &Config) -> String {
    let main_pac =
        config.proxy.main.as_deref().map(format_pac_proxy).unwrap_or_else(|| "DIRECT".to_string());
    let fallback_pac = config
        .proxy
        .fallback
        .as_deref()
        .map(format_pac_proxy)
        .unwrap_or_else(|| "DIRECT".to_string());

    // Construct the standard "on" route: Main -> Fallback -> Direct
    let mut on_route_parts = Vec::new();
    if main_pac != "DIRECT" {
        on_route_parts.push(main_pac.clone());
    }
    if fallback_pac != "DIRECT" {
        on_route_parts.push(fallback_pac.clone());
    }
    on_route_parts.push("DIRECT".to_string());
    let on_route = on_route_parts.join("; ");

    let mut script = String::new();
    script.push_str("function FindProxyForURL(url, host) {\n");

    // Process specific rules
    let mut sorted_domains: Vec<_> = config.rules.keys().collect();
    sorted_domains.sort_by_key(|d| std::cmp::Reverse(d.len())); // Match specific (longer) domains first

    for domain in sorted_domains {
        if domain == "*" {
            continue;
        }
        let rule = &config.rules[domain];

        let condition = if domain.starts_with("*.") {
            format!("shExpMatch(host, \"{}\")", domain)
        } else {
            format!("host === \"{}\"", domain)
        };

        // 1. Override has highest priority
        if let Some(over) = &rule.override_action {
            script.push_str(&format!(
                "  if ({}) return \"{}\";\n",
                condition,
                format_pac_proxy(over)
            ));
            continue;
        }

        // 2. Proxy On/Off
        if let Some(p) = &rule.proxy {
            if p.eq_ignore_ascii_case("on") {
                script.push_str(&format!("  if ({}) return \"{}\";\n", condition, on_route));
                continue;
            } else if p.eq_ignore_ascii_case("off") {
                script.push_str(&format!("  if ({}) return \"DIRECT\";\n", condition));
                continue;
            }
        }

        // 3. Custom Fallback for this rule
        if let Some(f) = &rule.fallback {
            let rule_fallback = format_pac_proxy(f);
            let mut chain = vec![main_pac.clone()];
            if rule_fallback != "DIRECT" {
                chain.push(rule_fallback);
            }
            chain.push("DIRECT".to_string());
            script.push_str(&format!("  if ({}) return \"{}\";\n", condition, chain.join("; ")));
        }
    }

    // Default behavior (if no rules matched)
    // Handle "*" rule as global default
    if let Some(global_rule) = config.rules.get("*") {
        if let Some(over) = &global_rule.override_action {
            script.push_str(&format!("  return \"{}\";\n", format_pac_proxy(over)));
        } else if let Some(p) = &global_rule.proxy {
            if p.eq_ignore_ascii_case("off") {
                script.push_str("  return \"DIRECT\";\n");
            } else {
                script.push_str(&format!("  return \"{}\";\n", on_route));
            }
        } else {
            script.push_str(&format!("  return \"{}\";\n", on_route));
        }
    } else {
        // Absolute default: Use the main/fallback chain
        script.push_str(&format!("  return \"{}\";\n", on_route));
    }

    script.push_str("}\n");
    script
}
