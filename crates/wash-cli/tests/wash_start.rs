use anyhow::Result;
use serial_test::serial;

mod common;
use common::{TestWashInstance, PROVIDER_HTTPSERVER_OCI_REF};

#[tokio::test]
#[serial]
async fn integration_start_stop_provider_serial() -> Result<()> {
    let wash_instance = TestWashInstance::create().await?;

    wash_instance
        .start_provider(PROVIDER_HTTPSERVER_OCI_REF)
        .await?;

    // Test stopping using only aliases, yes I know this mixes stop and start, but saves on copied
    // code
    wash_instance
        .stop_provider("server", "wasmcloud:httpserver", None, None)
        .await?;

    Ok(())
}
