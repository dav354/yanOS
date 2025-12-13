use std::time::Duration;
use tokio::sync::{broadcast, mpsc};
use yanos_backend::actors::MetricsActor;

#[tokio::test]
async fn test_metrics_collection_and_broadcast() {
    // Setup
    let (_cmd_tx, _cmd_rx) = mpsc::channel(10);
    let (broadcast_tx, mut broadcast_rx) = broadcast::channel(10);

    // Start Actor
    let actor = MetricsActor::new(_cmd_rx, broadcast_tx);
    tokio::spawn(actor.run());

    // Wait for at least one tick (Actor ticks every 1s)
    // We wait 1.5s to be safe
    tokio::time::sleep(Duration::from_millis(1500)).await;

    // Verify broadcast received
    match broadcast_rx.try_recv() {
        Ok(point) => {
            // Check data validity
            assert!(point.ts > 0);
            assert!(point.memory_total > 0);
            // CPU usage could be 0, but it should be a valid float
            assert!(point.cpu_user >= 0.0);
            assert!(point.cpu_idle >= 0.0);

            println!("Received metric: {:?}", point);
        }
        Err(e) => {
            panic!("Failed to receive metric from broadcast: {:?}", e);
        }
    }
}
