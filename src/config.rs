use std::collections::HashMap;

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
    pub proxy: Option<String>,
    pub fallback: Option<String>,
}

use once_cell::sync::Lazy;

pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    let content = std::fs::read_to_string("drover.toml").unwrap_or_default();
    toml::from_str(&content).unwrap_or_default()
});

fn format_pac_proxy(addr: &str) -> String {
    if addr.eq_ignore_ascii_case("DIRECT") {
        return "DIRECT".to_string();
    }
    // Handle cases where the user might have already put PROXY/SOCKS
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
        // If no prefix, check if it looks like an IP:PORT and default to PROXY
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

    let on_route = if !fallback_pac.eq_ignore_ascii_case("DIRECT") {
        format!("{}; {}; DIRECT", main_pac, fallback_pac)
    } else {
        format!("{}; DIRECT", main_pac)
    };

    let mut script = String::new();
    script.push_str("function FindProxyForURL(url, host) {\n");

    // Check for global rule "*"
    if let Some(rule) = config.rules.get("*") {
        if let Some(over) = &rule.override_action {
            script.push_str(&format!("  return \"{}\";\n", format_pac_proxy(over)));
            script.push_str("}\n");
            return script;
        }
        if let Some(proxy_val) = &rule.proxy {
            if proxy_val.eq_ignore_ascii_case("on") {
                script.push_str(&format!("  return \"{}\";\n", on_route));
                script.push_str("}\n");
                return script;
            }
        }
    }

    // Individual rules
    for (domain, rule) in &config.rules {
        if domain == "*" {
            continue;
        }

        let condition = if domain.starts_with("*.") {
            format!("shExpMatch(host, \"{}\")", domain)
        } else {
            format!("host === \"{}\"", domain)
        };

        if let Some(over) = &rule.override_action {
            script.push_str(&format!(
                "  if ({}) return \"{}\";\n",
                condition,
                format_pac_proxy(over)
            ));
            continue;
        }

        if let Some(proxy_val) = &rule.proxy {
            if proxy_val.eq_ignore_ascii_case("on") {
                script.push_str(&format!("  if ({}) return \"{}\";\n", condition, on_route));
                continue;
            } else if proxy_val.eq_ignore_ascii_case("off") {
                script.push_str(&format!("  if ({}) return \"DIRECT\";\n", condition));
                continue;
            }
        }

        if let Some(fallb) = &rule.fallback {
            script.push_str(&format!(
                "  if ({}) return \"{}; {}\";\n",
                condition,
                main_pac,
                format_pac_proxy(fallb)
            ));
        }
    }

    // Default: if main proxy is set, use it for everything else too?
    // User asked "why filter by domain". Let's make it the default if no rules matched.
    if !main_pac.eq_ignore_ascii_case("DIRECT") {
        script.push_str(&format!("  return \"{}\";\n", on_route));
    } else {
        script.push_str("  return \"DIRECT\";\n");
    }

    script.push_str("}\n");
    script
}
