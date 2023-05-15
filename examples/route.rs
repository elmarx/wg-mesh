use futures::TryStreamExt;
use rtnetlink::{new_connection, IpVersion, RouteHandle};

#[tokio::main]
async fn main() -> Result<(), rtnetlink::Error> {
    let (connection, handle, _) = new_connection().unwrap();
    tokio::spawn(connection);

    let mut x = RouteHandle::new(handle).get(IpVersion::V4).execute();

    while let Some(msg) = x.try_next().await? {
        println!("{msg:?}");
    }

    Ok(())
}
