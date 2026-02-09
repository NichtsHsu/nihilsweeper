use interprocess::local_socket::{GenericFilePath, GenericNamespaced, ListenerOptions, Name, Stream, prelude::*};
use log::{debug, error, info, warn};
use std::{
    io::{Read, Write},
    sync::{Arc, OnceLock},
};

const SOCKET_NAME: &str = "nihilsweeper-ipc";
const ACTIVATION_MSG: &[u8] = b"ACTIVATE";

// Global notifier for window activation
static ACTIVATION_NOTIFIER: OnceLock<Arc<tokio::sync::Notify>> = OnceLock::new();

// Global single instance lock - must be kept alive for the entire application lifetime
static SINGLE_INSTANCE_LOCK: OnceLock<single_instance::SingleInstance> = OnceLock::new();

/// Get the activation notifier (initializes it if not already initialized)
fn get_activation_notifier() -> Arc<tokio::sync::Notify> {
    ACTIVATION_NOTIFIER
        .get_or_init(|| Arc::new(tokio::sync::Notify::new()))
        .clone()
}

/// Creates a subscription that listens for activation requests
pub fn activation_subscription() -> iced::Subscription<crate::ui::AppMessage> {
    struct ActivationSubscription;

    iced::Subscription::run_with(std::any::TypeId::of::<ActivationSubscription>(), |_| {
        let notifier = get_activation_notifier();
        iced::futures::stream::unfold(notifier, |notifier| async move {
            notifier.notified().await;
            Some((crate::ui::AppMessage::ActivateWindow, notifier))
        })
    })
}

/// Checks if this is the first instance of the application.
/// If it is, returns Ok(true) and starts the IPC server.
/// If it's not the first instance, sends an activation message to the first instance and returns
/// Ok(false).
pub fn check_single_instance() -> Result<bool, Box<dyn std::error::Error>> {
    let instance = single_instance::SingleInstance::new("nihilsweeper-app")?;

    if instance.is_single() {
        info!("This is the first instance, setting up IPC server");

        // Store the instance lock in a static to keep it alive for the application lifetime
        // This prevents other instances from acquiring the lock
        SINGLE_INSTANCE_LOCK
            .set(instance)
            .map_err(|_| "Failed to store single instance lock")?;

        let notifier = get_activation_notifier();

        // Start IPC server in a background thread
        std::thread::spawn(move || {
            if let Err(e) = run_ipc_server(notifier) {
                error!("IPC server error: {}", e);
            }
        });

        Ok(true)
    } else {
        info!("Another instance is already running, sending activation message");
        if let Err(e) = send_activation_message() {
            warn!("Failed to send activation message: {}", e);
        }
        Ok(false)
    }
}

fn run_ipc_server(notifier: Arc<tokio::sync::Notify>) -> Result<(), Box<dyn std::error::Error>> {
    let name = get_socket_name()?;
    debug!("Creating IPC server with socket name: {:?}", name);

    // Clean up any existing socket file to prevent "Address already in use" error
    // This is necessary for file-based Unix domain sockets
    if !GenericNamespaced::is_supported() {
        // We're using file-based socket, remove it if it exists
        let path = format!("/tmp/{}.sock", SOCKET_NAME);
        if std::path::Path::new(&path).exists() {
            debug!("Removing existing socket file: {}", path);
            let _ = std::fs::remove_file(&path);
        }
    }

    let listener = ListenerOptions::new().name(name).create_sync()?;
    info!("IPC server listening for activation requests");

    for connection in listener.incoming() {
        match connection {
            Ok(mut stream) => {
                let mut buffer = [0u8; 64];
                match stream.read(&mut buffer) {
                    Ok(n) if n > 0 => {
                        if &buffer[..n] == ACTIVATION_MSG {
                            info!("Received activation request");
                            notifier.notify_one();
                        }
                    },
                    Ok(_) => {},
                    Err(e) => warn!("Failed to read from IPC stream: {}", e),
                }
            },
            Err(e) => warn!("Failed to accept IPC connection: {}", e),
        }
    }

    Ok(())
}

fn send_activation_message() -> Result<(), Box<dyn std::error::Error>> {
    let name = get_socket_name()?;
    debug!("Connecting to IPC server with socket name: {:?}", name);

    let mut stream = Stream::connect(name)?;
    stream.write_all(ACTIVATION_MSG)?;
    stream.flush()?;
    info!("Activation message sent successfully");

    Ok(())
}

fn get_socket_name() -> Result<Name<'static>, Box<dyn std::error::Error>> {
    if GenericNamespaced::is_supported() {
        // Abstract namespace socket (Linux) or Windows named pipe
        Ok(SOCKET_NAME.to_ns_name::<GenericNamespaced>()?)
    } else {
        // Unix domain socket with file path
        let path = format!("/tmp/{}.sock", SOCKET_NAME);
        Ok(path.to_fs_name::<GenericFilePath>()?)
    }
}
