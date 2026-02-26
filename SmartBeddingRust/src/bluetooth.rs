use bluer::{
    adv::Advertisement,
    agent::Agent,
    AdapterEvent,
};
use futures_util::StreamExt;

pub async fn run_bluetooth_service() -> bluer::Result<()> {
    let session = bluer::Session::new().await?;
    let adapter = session.default_adapter().await?;
    adapter.set_powered(true).await?;
    adapter.set_alias("SmartBeddingBT".to_string()).await?;
    adapter.set_discoverable(true).await?;

    // Definimos el agente más básico posible. 
    // Al no tener campos, usa los valores por defecto que no bloquean.
    let agent = Agent::default();
    
    // Registramos el agente.
    let _agent_handle = session.register_agent(agent).await?;

    let adv = Advertisement {
        local_name: Some("SmartBeddingBT".to_string()),
        discoverable: Some(true),
        ..Default::default()
    };
    let _adv_handle = adapter.advertise(adv).await?;

    println!("Bluetooth: Servicio activo como 'SmartBeddingBT'.");

    let mut events = adapter.events().await?;
    while let Some(event) = events.next().await {
        if let AdapterEvent::DeviceAdded(addr) = event {
            println!("Bluetooth: Conexión detectada desde: {}", addr);
        }
    }

    Ok(())
}