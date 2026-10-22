//! Calling a desktop portal (xdg-desktop-portal) over D-Bus through zbus, the crate and the version
//! that GPUI uses: the print portal and the file chooser. A portal answers a call with the path of a
//! request object and sends the result later as its `Response` signal.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};

use zbus::export::futures_core::Stream;
use zbus::export::serde::Serialize;
use zbus::zvariant::{DynamicType, OwnedValue, Value};
use zbus::{Connection, Proxy};

const PORTAL: &str = "org.freedesktop.portal.Desktop";
const PORTAL_PATH: &str = "/org/freedesktop/portal/desktop";

/// How the portal answered a request (the Response signal of org.freedesktop.portal.Request).
pub(crate) enum Answer {
    Done(HashMap<String, OwnedValue>),
    Cancelled,
    /// The interaction ended in some other way.
    Failed,
}

/// Why a request failed: the portal did not take the call (there is none, or none with the interface),
/// or its answer did not come or could not be read.
pub(crate) enum Failure {
    Call(String),
    Answer(String),
}

pub(crate) struct Portal {
    connection: Connection,
    proxy: Proxy<'static>,
}

impl Portal {
    /// Connects to the session bus and the portal interface `interface`
    /// (`org.freedesktop.portal.Print`, `org.freedesktop.portal.FileChooser`).
    pub(crate) async fn new(interface: &'static str) -> Result<Self, String> {
        let connection = Connection::session().await.map_err(|e| e.to_string())?;
        let proxy = Proxy::new(&connection, PORTAL, PORTAL_PATH, interface)
            .await
            .map_err(|e| e.to_string())?;
        Ok(Self { connection, proxy })
    }

    /// Calls a method of the portal and waits for its answer. `body` makes the arguments
    /// from the options, which hold the token of the request.
    pub(crate) async fn request<B>(
        &self,
        method: &'static str,
        body: impl FnOnce(HashMap<&'static str, Value<'static>>) -> B,
    ) -> Result<Answer, Failure>
    where
        B: Serialize + DynamicType,
    {
        let call = |err: zbus::Error| Failure::Call(err.to_string());
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let token = format!(
            "matsu_{}_{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        // The portal answers on an object named after the caller and the token, so the answer is
        // awaited before the call, not to miss it.
        let sender = self
            .connection
            .unique_name()
            .ok_or_else(|| Failure::Call("D-Bus の接続に名前がありません".into()))?
            .trim_start_matches(':')
            .replace('.', "_");
        let path = format!("{PORTAL_PATH}/request/{sender}/{token}");
        let request = Proxy::new(
            &self.connection,
            PORTAL,
            path,
            "org.freedesktop.portal.Request",
        )
        .await
        .map_err(call)?;
        let mut answers = std::pin::pin!(request.receive_signal("Response").await.map_err(call)?);
        let mut options = HashMap::new();
        options.insert("handle_token", Value::from(token));
        self.proxy
            .call_method(method, &body(options))
            .await
            .map_err(call)?;
        let answer = std::future::poll_fn(|cx| answers.as_mut().poll_next(cx))
            .await
            .ok_or_else(|| Failure::Answer("ポータルが応答しません".into()))?;
        let (code, results): (u32, HashMap<String, OwnedValue>) = answer
            .body()
            .deserialize()
            .map_err(|err| Failure::Answer(err.to_string()))?;
        Ok(match code {
            0 => Answer::Done(results),
            1 => Answer::Cancelled,
            _ => Answer::Failed,
        })
    }
}
