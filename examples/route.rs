use futures::TryStreamExt;
use rtnetlink::{RouteMessageBuilder, new_connection};
use std::net::Ipv4Addr;

#[tokio::main]
async fn main() -> Result<(), rtnetlink::Error> {
    let (route_connection, handle, _) = new_connection().unwrap();
    let (link_connection, _link_handle, _) = new_connection().unwrap();
    tokio::spawn(route_connection);
    tokio::spawn(link_connection);

    let route_handle = handle.route();
    let link_handle = handle.link();

    let routes: Vec<_> = route_handle
        .get(RouteMessageBuilder::<Ipv4Addr>::new().build())
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

    println!("{link:?}");
    println!("{routes:?}");

    Ok(())
}
