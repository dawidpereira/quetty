use app::model::Model;
use components::common::ComponentId;
use server::service_bus_manager::ServiceBusManager;
use tuirealm::application::PollStrategy;
use tuirealm::{AttrValue, Attribute, Update};

mod app;
mod components;
mod config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup model
    let mut model = Model::new().await;

    // Use AzureAdConfig from config
    let azure_config = config::CONFIG.azure_ad().clone();

    // Fetch queues
    let queues = ServiceBusManager::list_queues_azure_ad(&azure_config)
        .await
        .unwrap_or_else(|_| vec![]);

    model.remount_queue_picker(queues);

    // Enter alternate screen
    let _ = model.terminal.enter_alternate_screen();
    let _ = model.terminal.enable_raw_mode();

    // Main loop
    while !model.quit {
        model.update_outside_msg();
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
