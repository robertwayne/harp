/// This macro is a helper method to simplify the initial setup of a Harp
/// service. It will attempt to connect to the Harp server and, if successful,
/// will spawn a new task via Tokio to run the service.
///
/// The returned `flume::Sender` is cheaply cloneable, so you can pass it among
/// threads and tasks.
///
/// # Examples
///
/// ```
/// # #[macro_use] extern crate harp;
/// #
/// # use harp::{Harp, action::{Action, Kind}, Loggable, HarpId};
/// # use std::net::{IpAddr, Ipv4Addr};
/// #
/// # pub struct MyAction {}
/// #
/// # impl Loggable for MyAction {
/// #     fn identifier(&self) -> HarpId {
/// #         (IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0)}
/// # }
/// #
/// # pub enum MyKind {
/// #     A
/// # }
/// #
/// # impl Kind for MyKind {
/// #     fn key(&self) -> &'static str {
/// #         match self {
/// #           MyKind::A => "a"
/// #         }
/// #     }
/// # }
/// #
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // The return value here is `flume::Sender<harp::action::Action>`
/// let harp = harp::create_service!();
///
/// // We can then create an action...
/// // See the `action::Action` documentation for more information on
/// // constructing actions and implementing the Loggable trait.
/// let action = Action::new(MyKind::A, &MyAction{});
///
/// // ...and send it to the Harp server.
/// harp.send(action)?;
/// #
/// #     Ok(())
/// # }
/// ```
///
/// You can also specify a custom hostname and port:
///       
/// ```
/// # #[macro_use] extern crate harp;
/// #
/// # use harp::{Harp, action::Action, Loggable, HarpId};
/// # use std::net::{IpAddr, Ipv4Addr};
/// #
/// # pub struct MyAction {}
/// #
/// # impl Loggable for MyAction {
/// #     fn identifier(&self) -> HarpId {
/// #         (IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0)}
/// # }
/// #
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let harp = harp::create_service!("127.0.0.1", 7778);
/// # Ok(())
/// # }
/// ```              
#[macro_export]
macro_rules! create_service {
    () => {{
        let mut harp = Harp::connect().await?;
        let tx = harp.get_sender();

        tokio::spawn(async move {
            let _ = harp.run().await;
        });

        tx
    }};
    ($host:expr, $port:expr) => {{
        let mut harp = Harp::connect_with_options($host, $port).await?;
        let tx = harp.get_sender();

        tokio::spawn(async move {
            let _ = harp.run().await;
        });

        tx
    }};
}
