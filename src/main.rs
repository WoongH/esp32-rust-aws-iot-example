//! Example of using blocking wifi.
//!
//! Add your own ssid and password

use embedded_svc::wifi::{AuthMethod, ClientConfiguration, Configuration};
use esp_idf_hal::prelude::Peripherals;
use esp_idf_svc::log::EspLogger;
use esp_idf_svc::wifi::{BlockingWifi, EspWifi};
use esp_idf_svc::{eventloop::EspSystemEventLoop, nvs::EspDefaultNvsPartition};
use esp_idf_sys::{self as _}; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use log::info;

// Import the necessary libraries
use embedded_svc::mqtt::client::{Connection, MessageImpl, QoS};
use embedded_svc::utils::mqtt::client::{ConnState};

use esp_idf_svc::mqtt::client::{EspMqttClient, MqttClientConfiguration};
use esp_idf_svc::tls::X509;
use esp_idf_sys::EspError;

use std::{mem, slice};

// Define the AWS IoT endpoint, client ID and topic
const AWS_IOT_ENDPOINT: &str = "mqtts://<your-endpoint>";
const AWS_IOT_CLIENT_ID: &str = "<your-client-id>";
const AWS_IOT_TOPIC: &str = "<your-topic>";

// Wifi credentials
const SSID: &'static str = "WIFI_SSID";
const PASSWORD: &'static str = "WIFI_PASS";

fn main() -> anyhow::Result<()> {
    EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs))?,
        sys_loop,
    )?;

    connect_wifi(&mut wifi)?;

    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;

    info!("Wifi DHCP info: {:?}", ip_info);

    // Create an MQTT client and connection
    let mut client = create_mqtt_client()?;
    
    // Subscribe to the AWS IoT topic with QoS 1
    let result = client.subscribe(AWS_IOT_TOPIC, QoS::AtLeastOnce)?;
    if result == 0 {
        info!("client.subscribe Ok");
    } else {
        info!("client.subscribe Err");
    }

    let mut cnt = 0;
    loop {
        // Publish a message to the AWS IoT topic with QoS 1
        info!("Publish a message");
        let msg = format!("Hello from ESP32 - {}", cnt);
        client.publish(AWS_IOT_TOPIC, QoS::AtLeastOnce, false, msg.as_bytes())?;
        cnt += 1;
        std::thread::sleep(core::time::Duration::from_secs(10));
    }

    Ok(())
}


fn connect_wifi(wifi: &mut BlockingWifi<EspWifi<'static>>) -> anyhow::Result<()> {
    let wifi_configuration: Configuration = Configuration::Client(ClientConfiguration {
        ssid: SSID.into(),
        bssid: None,
        auth_method: AuthMethod::WPA2Personal,
        password: PASSWORD.into(),
        channel: None,
    });

    wifi.set_configuration(&wifi_configuration)?;

    wifi.start()?;
    info!("Wifi started");

    wifi.connect()?;
    info!("Wifi connected");

    wifi.wait_netif_up()?;
    info!("Wifi netif up");

    Ok(())
}

// Define a function to convert certificates to X509 format
fn convert_certificate(mut certificate_bytes: Vec<u8>) -> X509<'static> {
    // Append NUL
    certificate_bytes.push(0);

    // Convert the certificate
    let certificate_slice: &[u8] = unsafe {
        let ptr: *const u8 = certificate_bytes.as_ptr();
        let len: usize = certificate_bytes.len();
        mem::forget(certificate_bytes);

        slice::from_raw_parts(ptr, len)
    };

    // Return the certificate file in the correct format
    X509::pem_until_nul(certificate_slice)
}

// Define a function to create an MQTT client and connection
fn create_mqtt_client() -> Result<EspMqttClient<ConnState<MessageImpl, EspError>>, EspError> {
    // Load the certificates from files
    let server_cert_bytes: Vec<u8> = include_bytes!("certificates/AmazonRootCA1.pem").to_vec();
    let client_cert_bytes: Vec<u8> = include_bytes!("certificates/DeviceCertificate.pem").to_vec();
    let private_key_bytes: Vec<u8> = include_bytes!("certificates/client.private.key").to_vec();

    // Convert the certificates to X509 format
    let server_cert: X509 = convert_certificate(server_cert_bytes);
    let client_cert: X509 = convert_certificate(client_cert_bytes);
    let private_key: X509 = convert_certificate(private_key_bytes);

    // Create an MQTT client configuration with TLS and certificates
    let conf = MqttClientConfiguration {
        client_id: Some(AWS_IOT_CLIENT_ID),
        crt_bundle_attach: Some(esp_idf_sys::esp_crt_bundle_attach),
        server_certificate: Some(server_cert),
        client_certificate: Some(client_cert),
        private_key: Some(private_key),
        ..Default::default()
    };

    let (client, mut connection) = EspMqttClient::new_with_conn(AWS_IOT_ENDPOINT, &conf)?;
    info!("MQTT client started");

    std::thread::spawn(move || {
        info!("std::thread - MQTT Listening for messages");

        while let Some(msg) = connection.next() {
            match msg {
                Err(e) => info!("MQTT Message ERROR: {}", e),
                Ok(msg) => info!("MQTT Message: {:?}", msg),
            }
        }
    });


    Ok(client)
    
}

#[no_mangle]
pub extern "C" fn app_main() {
    main().unwrap();
}
