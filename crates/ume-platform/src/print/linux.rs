//! Printing on Linux (ADR-0011): the print portal (org.freedesktop.portal.Print) shows the
//! desktop's print dialog and prints a PDF of the pages (krilla). GTK's print dialog is LGPL, so
//! it is not used directly. Ported from the PoC of 2026-10-11 (`poc/gpui-textview/src/print/linux.rs`).
//!
//! The portal is called through zbus, not ashpd's print proxy: ashpd reads the settings of the
//! dialog into enums and fails on what GTK's dialog gives for printing to a file (its
//! `output-file-format` is `pdf`, where ashpd 0.13.13 takes only `PDF`). Only the paper and the
//! token are read here.

use std::collections::HashMap;
use std::io::{Seek, Write};
use std::os::fd::AsFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use zbus::export::futures_core::Stream;
use zbus::export::serde::Serialize;
use zbus::zvariant::{DynamicType, Fd, OwnedValue, Value};
use zbus::{Connection, Proxy};

use ume_print::Paper;

use super::{Error, PrintJob, pdf};

const PORTAL: &str = "org.freedesktop.portal.Desktop";
const PORTAL_PATH: &str = "/org/freedesktop/portal/desktop";

/// Shows the print dialog and prints. Returns whether it printed: false when the user cancels.
/// Without the portal, the PDF is saved instead and the error says where, so that the pages can
/// still be checked.
pub async fn print(job: PrintJob) -> Result<bool, Error> {
    let title = job.title.as_str();
    let portal = match Portal::new().await {
        Ok(portal) => portal,
        Err(err) => return Err(save_instead(&job, &err)),
    };
    // The dialog is not attached to the window: that would take the window's Wayland or X11
    // handle, which the PoC does not ask GPUI for.
    let prepared = portal
        .request("PreparePrint", |mut options| {
            options.insert("modal", true.into());
            let none = HashMap::<&str, Value>::new;
            ("", title, none(), none(), options)
        })
        .await;
    let mut results = match prepared {
        // Before any dialog: no portal, or none that prints.
        Err(Failure::Call(err)) => return Err(save_instead(&job, &err)),
        Err(Failure::Answer(err)) => return Err(Error(err)),
        Ok(Answer::Cancelled) => return Ok(false),
        Ok(Answer::Failed) => return Err("印刷ダイアログを出せません".into()),
        Ok(Answer::Done(results)) => results,
    };
    let token = results
        .get("token")
        .and_then(|token| token.downcast_ref::<u32>().ok())
        .ok_or("印刷のポータルの応答に token がありません")?;
    let paper = results
        .remove("page-setup")
        .and_then(|setup| HashMap::<String, OwnedValue>::try_from(setup).ok())
        .and_then(|setup| paper_of(&setup))
        .unwrap_or_else(|| job.setting_paper());
    let (pdf, _) = pdf::render(&job, paper)?;
    // The portal reads the PDF from a file descriptor: a file without a name, which no other
    // process can open.
    let mut file = tempfile::tempfile().map_err(|e| Error(e.to_string()))?;
    file.write_all(&pdf)
        .and_then(|()| file.rewind())
        .map_err(|e| Error(e.to_string()))?;
    let printed = portal
        .request("Print", |mut options| {
            options.insert("token", token.into());
            ("", title, Fd::from(file.as_fd()), options)
        })
        .await
        .map_err(Error::from)?;
    match printed {
        Answer::Done(_) => Ok(true),
        Answer::Cancelled => Ok(false),
        Answer::Failed => Err("印刷できませんでした".into()),
    }
}

/// How the portal answered a request (the Response signal of org.freedesktop.portal.Request).
enum Answer {
    Done(HashMap<String, OwnedValue>),
    Cancelled,
    /// The interaction ended in some other way.
    Failed,
}

/// Why a request failed: the portal did not take the call (there is none, or none that prints),
/// or its answer did not come or could not be read.
enum Failure {
    Call(String),
    Answer(String),
}

impl From<Failure> for Error {
    fn from(failure: Failure) -> Self {
        match failure {
            Failure::Call(err) | Failure::Answer(err) => Error(err),
        }
    }
}

struct Portal {
    connection: Connection,
    print: Proxy<'static>,
}

impl Portal {
    async fn new() -> Result<Self, String> {
        let connection = Connection::session().await.map_err(|e| e.to_string())?;
        let print = Proxy::new(
            &connection,
            PORTAL,
            PORTAL_PATH,
            "org.freedesktop.portal.Print",
        )
        .await
        .map_err(|e| e.to_string())?;
        Ok(Self { connection, print })
    }

    /// Calls a method of the print portal and waits for its answer. `body` makes the arguments
    /// from the options, which hold the token of the request.
    async fn request<B>(
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
            "ume_{}_{}",
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
        self.print
            .call_method(method, &body(options))
            .await
            .map_err(call)?;
        let answer = std::future::poll_fn(|cx| answers.as_mut().poll_next(cx))
            .await
            .ok_or_else(|| Failure::Answer("印刷のポータルが応答しません".into()))?;
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

/// Saves the PDF in the temporary directory when the portal cannot print it, and says where.
fn save_instead(job: &PrintJob, reason: &str) -> Error {
    let saved = pdf::render(job, job.setting_paper()).and_then(|(pdf, _)| save_pdf(&pdf));
    Error(match saved {
        Ok(path) => format!(
            "印刷のポータルを使えません（{reason}）。PDF を {} に保存しました",
            path.display()
        ),
        Err(err) => format!("印刷のポータルを使えません（{reason}）。PDF も保存できません: {err}"),
    })
}

/// Writes the PDF to a new file that only the user can read, with a name no one can take first.
fn save_pdf(pdf: &[u8]) -> Result<PathBuf, Error> {
    let mut file = tempfile::Builder::new()
        .prefix("ume-print-")
        .suffix(".pdf")
        .tempfile()
        .map_err(|e| Error(e.to_string()))?;
    file.write_all(pdf).map_err(|e| Error(e.to_string()))?;
    let (_, path) = file.keep().map_err(|e| Error(e.to_string()))?;
    Ok(path)
}

/// The paper chosen in the dialog, as it is turned. The portal gives its size in millimetres,
/// upright, and the orientation apart (`portrait`, `landscape`, `reverse_portrait` or
/// `reverse_landscape`, as GTK names them).
fn paper_of(setup: &HashMap<String, OwnedValue>) -> Option<Paper> {
    let millimetres = |key: &str| setup.get(key)?.downcast_ref::<f64>().ok();
    let (width, height) = (millimetres("Width")?, millimetres("Height")?);
    if width <= 0.0 || height <= 0.0 {
        return None;
    }
    let (width, height) = (
        (width * 10.0).round() as i32,
        (height * 10.0).round() as i32,
    );
    let landscape = setup
        .get("Orientation")
        .and_then(|orientation| orientation.downcast_ref::<String>().ok())
        .is_some_and(|orientation| orientation.ends_with("landscape"));
    Some(if landscape {
        Paper {
            width: height,
            height: width,
        }
    } else {
        Paper { width, height }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup(width: f64, height: f64, orientation: Option<&str>) -> HashMap<String, OwnedValue> {
        let mut map = HashMap::new();
        map.insert("Width".to_owned(), Value::from(width).try_into().unwrap());
        map.insert("Height".to_owned(), Value::from(height).try_into().unwrap());
        if let Some(orientation) = orientation {
            map.insert(
                "Orientation".to_owned(),
                Value::from(orientation).try_into().unwrap(),
            );
        }
        map
    }

    #[test]
    fn the_paper_of_the_dialog_is_turned_by_its_orientation() {
        // The portal gives the upright size in mm; Sakura's lengths are 0.1 mm.
        let a4 = Paper {
            width: 2100,
            height: 2970,
        };
        assert_eq!(paper_of(&setup(210.0, 297.0, Some("portrait"))), Some(a4));
        assert_eq!(paper_of(&setup(210.0, 297.0, None)), Some(a4));
        assert_eq!(
            paper_of(&setup(210.0, 297.0, Some("reverse_portrait"))),
            Some(a4)
        );
        let turned = Paper {
            width: 2970,
            height: 2100,
        };
        assert_eq!(
            paper_of(&setup(210.0, 297.0, Some("landscape"))),
            Some(turned)
        );
        assert_eq!(
            paper_of(&setup(210.0, 297.0, Some("reverse_landscape"))),
            Some(turned)
        );
    }

    #[test]
    fn a_paper_without_a_size_is_not_taken() {
        assert_eq!(paper_of(&HashMap::new()), None);
        assert_eq!(paper_of(&setup(0.0, 297.0, None)), None);
    }
}
