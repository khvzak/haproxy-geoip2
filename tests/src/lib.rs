#![cfg(test)]

use serde_json::Value as JsonValue;

#[test]
fn integration_tests() {
    // Compile haproxy-geoip2-module
    std::process::Command::new("cargo")
        .args(&["build", "--release", "-p", "haproxy-geoip2-module"])
        .current_dir("..")
        .status()
        .expect("Failed to compile haproxy-geoip2-module");

    // Spawn haproxy and wait
    let mut haproxy = std::process::Command::new("haproxy")
        .args(&["-f", "haproxy.cfg"])
        .spawn()
        .expect("Failed to start haproxy");
    std::thread::sleep(std::time::Duration::from_secs(1));

    run_tests().unwrap();

    haproxy.kill().expect("Failed to stop haproxy");
}

fn run_tests() -> Result<(), Box<dyn std::error::Error>> {
    let res = reqwest::blocking::get("http://localhost:8080?ip=216.160.83.56")?;
    let body = res.json::<JsonValue>()?;
    assert_eq!(body["asn"], "209");
    assert_eq!(body["city"], "Milton");

    let res = reqwest::blocking::get("http://localhost:8080?ip=89.160.20.112")?;
    let body = res.json::<JsonValue>()?;
    assert_eq!(body["asn"], "29518");
    assert_eq!(body["city"], "Linköping");

    Ok(())
}
