use app::model::Model;
//NOTE: Consider whether it is worth removing the reference to AzureServiceBus from the UI model
use azservicebus::{ServiceBusClient, ServiceBusClientOptions, ServiceBusReceiverOptions};
use components::common::ComponentId;
use server::consumer::ServiceBusClientExt;
use tuirealm::application::PollStrategy;
use tuirealm::{AttrValue, Attribute, Update};

mod app;
mod components;
mod config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup model
    let mut client = ServiceBusClient::new_from_connection_string(
        config::CONFIG.servicebus().connection_string(),
        ServiceBusClientOptions::default(),
    )
    .await?;

    //TODO: Get messages after messages view loading. Blocked until queu switcher
    let mut receiver = client
        .create_consumer_for_queue(
            config::CONFIG.servicebus().queue_name(),
            ServiceBusReceiverOptions::default(),
        )
        .await?;

    let messages = receiver
        .peek_messages(config::CONFIG.max_messages(), None)
        .await
        .unwrap();

    let mut model = Model::new_crossterm(Some(messages));

    // Enter alternate screen
    let _ = model.terminal.enter_alternate_screen();
    let _ = model.terminal.enable_raw_mode();

    // Main loop
    while !model.quit {
        // Tick
        match model.app.tick(PollStrategy::Once) {
            Err(err) => {
                assert!(
                    model
                        .app
                        .attr(
                            &ComponentId::Label,
                            Attribute::Text,
                            AttrValue::String(format!("Application error: {}", err)),
                        )
                        .is_ok()
                );
            }
            Ok(messages) if !messages.is_empty() => {
                // NOTE: redraw if at least one msg has been processed
                model.redraw = true;
                for msg in messages.into_iter() {
                    let mut msg = Some(msg);
                    while msg.is_some() {
                        msg = model.update(msg);
                    }
                }
            }
            _ => {}
        }
        // Redraw
        if model.redraw {
            model.view();
            model.redraw = false;
        }
    }
    // Terminate terminal
    let _ = model.terminal.leave_alternate_screen();
    let _ = model.terminal.disable_raw_mode();
    let _ = model.terminal.clear_screen();
    Ok(())
}
