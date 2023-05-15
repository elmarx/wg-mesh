use futures::TryStreamExt;
use rtnetlink::{new_connection, IpVersion, LinkHandle, RouteHandle};

#[tokio::main]
async fn main() -> Result<(), rtnetlink::Error> {
    let (route_connection, handle, _) = new_connection().unwrap();
    let (link_connection, link_handle, _) = new_connection().unwrap();
    tokio::spawn(route_connection);
    tokio::spawn(link_connection);

    let route_handle = handle.route();
    let mut link_handle = handle.link();

    let routes: Vec<_> = route_handle
        .get(IpVersion::V4)
        .execute()
        .try_collect()
        .await?;

    let link = link_handle
        .get()
        .match_name("wg0".to_string())
        .execute()
        .try_next()
        .await?
        .unwrap();

    let index = link.header.index;

    println!("{link:?}");
    println!("{routes:?}");

    Ok(())
}
