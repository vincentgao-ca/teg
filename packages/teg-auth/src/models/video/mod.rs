use std::sync::{Arc, Weak};

use async_std::prelude::*;

use gst::gst_element_error;
use gst::prelude::*;
use async_std::task;
use futures::stream::StreamExt;
use futures::future::FutureExt;

use anyhow::{anyhow, bail, Context};

// use chrono::prelude::*;
use juniper::{
    FieldResult,
    // FieldError,
};
use serde::{
    Serialize,
    Deserialize,
};

// use crate::ResultExt;

// use crate::models::{ Invite };

// const STUN_SERVER: &str = "stun://stun.l.google.com:19302";
const STUN_SERVER: &str = "stun://global.stun.twilio.com:3478?transport=udp";
// const TURN_SERVER: &str = "turn://foo:bar@webrtc.nirbheek.in:3478";

// upgrade weak reference or return
#[macro_export]
macro_rules! upgrade_weak {
    ($x:ident, $r:expr) => {{
        match $x.upgrade() {
            Some(o) => o,
            None => return $r,
        }
    }};
    ($x:ident) => {
        upgrade_weak!($x, ())
    };
}

#[derive(juniper::GraphQLObject, Debug, Serialize, Deserialize)]
pub struct RTCSignal {
    #[graphql(name = "type")]
    pub sdp_type: String,
    pub sdp: String,
}

#[derive(juniper::GraphQLInputObject, Debug, Serialize, Deserialize)]
pub struct RTCSignalInput {
    #[graphql(name = "type")]
    pub sdp_type: String,
    pub sdp: String,
}

pub async fn create_video_sdp(
    context: &crate::Context,
    offer: RTCSignalInput,
) -> FieldResult<RTCSignal> {
    // Initialize GStreamer first
    gst::init()
        .map_err(|err| format!("gst::init(): {:?}", err))?;

    check_plugins()
        .map_err(|err| format!("check_plugins(): {:?}", err))?;

    // Create our application state
    let (app, send_gst_msg_rx) = App::new()
        .await
        .map_err(|err| format!("Unable to start video provider: {:?}", err))?;

    // Receive the SDP
    let answer_future = app.handle_sdp(&offer.sdp_type, &offer.sdp)?;

    let app_clone = app.clone();
    let pipeline_message_handler = async move {
        let mut send_gst_msg_rx = send_gst_msg_rx.fuse();
        loop {
            // Pass the GStreamer messages to the application control logic
            let gst_msg = send_gst_msg_rx.select_next_some().await;
            let _ = app_clone.handle_pipeline_message(&gst_msg)
                .expect("Pipeline Error");
        }
    };

    let app_clone = app.clone();
    let sdp_handler = async move {
        let reply = answer_future.await
            .map_err(|err| format!("Failed to load video SDP: {:?}", err))?;

        info!("SDP reply ready: {:?}", reply);

        let answer = app_clone.on_answer_created(reply)
            .map_err(|err| {
                gst_element_error!(
                    app_clone.pipeline,
                    gst::LibraryError::Failed,
                    ("Failed to send SDP answer: {:?}", err)
                );
                format!("Failed to create video SDP answer: {:?}", err)
            })?;

        Ok(answer)
    };

    let answer = futures::select!{
        answer = sdp_handler.fuse() => answer,
        _ = pipeline_message_handler.fuse() => Err("Unexpected pipeline message handler exit".to_string()),
    }?;

    Ok(answer)
}

// Strong reference to our application state
#[derive(Debug, Clone)]
pub struct App(Arc<AppInner>);

// Weak reference to our application state
#[derive(Debug, Clone)]
struct AppWeak(Weak<AppInner>);

// Actual application state
#[derive(Debug)]
pub struct AppInner {
    pipeline: gst::Pipeline,
    webrtcbin: gst::Element,
}

// To be able to access the App's fields directly
impl std::ops::Deref for App {
    type Target = AppInner;

    fn deref(&self) -> &AppInner {
        &self.0
    }
}

impl AppWeak {
    // Try upgrading a weak reference to a strong one
    fn upgrade(&self) -> Option<App> {
        self.0.upgrade().map(App)
    }
}

impl App {
    // Downgrade the strong reference to a weak reference
    fn downgrade(&self) -> AppWeak {
        AppWeak(Arc::downgrade(&self.0))
    }

    pub async fn new(
    ) -> Result<
        (
            Self,
            impl Stream<Item = gst::Message>,
        ),
        anyhow::Error,
    > {
        let pipeline = gst::parse_launch(
            // Create the GStreamer pipeline (test pattern)
            "videotestsrc pattern=ball is-live=true ! vp8enc deadline=1 ! rtpvp8pay pt=96 ! webrtcbin. \
             audiotestsrc is-live=true ! opusenc ! rtpopuspay pt=97 ! webrtcbin. \
             webrtcbin name=webrtcbin"

            // Create the GStreamer pipeline (webcam pipeline)
            // https://gstreamer.freedesktop.org/documentation/video4linux2/v4l2src.html?gi-language=c
            // "v4l2src ! jpegdec ! vp8enc deadline=1 ! rtpvp8pay pt=96 ! webrtcbin. \
            //  audiotestsrc is-live=true volume=0 ! opusenc ! rtpopuspay pt=97 ! webrtcbin. \
            //  webrtcbin name=webrtcbin"
        )?;

        // Downcast from gst::Element to gst::Pipeline
        let pipeline = pipeline
            .downcast::<gst::Pipeline>()
            .expect("not a pipeline");

        // Get access to the webrtcbin by name
        let webrtcbin = pipeline
            .get_by_name("webrtcbin")
            .expect("can't find webrtcbin");

        // Set some properties on webrtcbin
        webrtcbin.set_property_from_str("stun-server", STUN_SERVER);
        // webrtcbin.set_property_from_str("turn-server", TURN_SERVER);
        webrtcbin.set_property_from_str("bundle-policy", "max-bundle");
        // webrtcbin.set_property_from_str("bundle-policy", "max-compat");

        // Create a stream for handling the GStreamer message asynchronously
        let bus = pipeline.get_bus().expect("Unable to connect to GStreamer");
        let send_gst_msg_rx = gst::BusStream::new(&bus);

        // Asynchronously set the pipeline to Playing
        pipeline.call_async(|pipeline| {
            pipeline
                .set_state(gst::State::Playing)
                .expect("Couldn't set pipeline to Playing");
        });

        let app = App(Arc::new(AppInner {
            pipeline,
            webrtcbin,
        }));

        // Asynchronously set the pipeline to Playing
        app.pipeline.call_async(|pipeline| {
            // If this fails, post an error on the bus so we exit
            if pipeline.set_state(gst::State::Playing).is_err() {
                gst_element_error!(
                    pipeline,
                    gst::LibraryError::Failed,
                    ("Failed to set pipeline to Playing")
                );
            }
        });

        // Whenever there is a new ICE candidate, send it to the peer
        let app_clone = app.downgrade();
        app.webrtcbin
            .connect("on-ice-candidate", false, move |values| {
                let _webrtc = values[0].get::<gst::Element>().expect("Invalid argument");
                let mlineindex = values[1].get_some::<u32>().expect("Invalid argument");
                let candidate = values[2]
                    .get::<String>()
                    .expect("Invalid argument")
                    .unwrap();

                let app = upgrade_weak!(app_clone, None);

                if let Err(err) = app.on_ice_candidate(mlineindex, candidate) {
                    gst_element_error!(
                        app.pipeline,
                        gst::LibraryError::Failed,
                        ("Failed to send ICE candidate: {:?}", err)
                    );
                }

                None
            })
            .unwrap();

        Ok((app, send_gst_msg_rx))
    }

    // Handle GStreamer messages coming from the pipeline
    pub fn handle_pipeline_message(&self, message: &gst::Message) -> Result<(), anyhow::Error> {
        use gst::message::MessageView;

        match message.view() {
            MessageView::Error(err) => bail!(
                "Error from element {}: {} ({})",
                err.get_src()
                    .map(|s| String::from(s.get_path_string()))
                    .unwrap_or_else(|| String::from("None")),
                err.get_error(),
                err.get_debug().unwrap_or_else(|| String::from("None")),
            ),
            MessageView::Warning(warning) => {
                let message = warning.get_debug().unwrap_or("[Warning Text Missing]".to_string());
                warn!("GStreamer Warning: \"{}\"", message);
            }
            _ => (),
        }

        Ok(())
    }

    // Asynchronously send ICE candidates to the peer via the WebSocket connection as a JSON
    // message
    fn on_ice_candidate(&self, mlineindex: u32, candidate: String) -> Result<(), anyhow::Error> {
        // let message = serde_json::to_string(&JsonMsg::Ice {
        //     candidate,
        //     sdp_mline_index: mlineindex,
        // })
        // .unwrap();

        info!("ICE Candidate ({:?}): {:?}", mlineindex, candidate);
        // self.send_msg_tx
        //     .lock()
        //     .unwrap()
        //     .unbounded_send(WsMessage::Text(message))
        //     .with_context(|| format!("Failed to send ICE candidate"))?;

        Ok(())
    }

    fn on_ice_gathering_state_change(
        &self,
        answer_promise: Arc<gst::promise::Promise>,
    ) -> Result<(), anyhow::Error> {
        let val = self.webrtcbin.get_property("ice-gathering-state")?
            .downcast::<gst_webrtc::WebRTCICEGatheringState>()
            .map_err(|e| anyhow!("unable to downcast gathering state: {:?}", e))?
            .get_some();

        info!("ICE Gathering State: {:?}", val);

        if let gst_webrtc::WebRTCICEGatheringState::Complete = val {
            use async_std::task;

            let app_clone = self.clone();

            task::spawn(async move {
                task::sleep(std::time::Duration::from_millis(500)).await;
                info!("Creating answer...");

                app_clone
                    .webrtcbin
                    .emit("create-answer", &[&None::<gst::Structure>, &*answer_promise])
                    .context("GStreamer Error in create-answer")
                    .unwrap();
            });
        };

        Ok(())
    }

    // Once webrtcbin has create the answer SDP for us, handle it by sending it to the peer via the
    // WebSocket connection
    fn on_answer_created(
        &self,
        reply: gst::Structure,
    ) -> Result<RTCSignal, anyhow::Error> {
        let answer = reply
            .get_value("answer")
            .context("Unable to get answer in reply")?
            .get::<gst_webrtc::WebRTCSessionDescription>()
            .context("Unable to get answer description")?
            .ok_or(anyhow!("Invalid argument"))?;

        info!("Answer {:?}", answer);

        self.webrtcbin
            .emit("set-local-description", &[&answer, &None::<gst::Promise>])
            .context("Unable to set local description")?;

        info!("local description set");

        let sdp = answer.get_sdp().as_text()
            .context("Unable to create SDP text")?;

        info!("sending SDP answer to peer");
        trace!("SDP Answer: {}", sdp);
        info!("SDP Answer: {}", sdp);

        let answer_signal = RTCSignal {
            sdp_type: "answer".to_string(),
            sdp,
        };

        Ok(answer_signal)
    }

    // Handle incoming SDP answers from the peer
    fn handle_sdp(
        &self,
        type_: &str,
        sdp: &str,
    ) -> Result<gst::promise::PromiseFuture, anyhow::Error> {
        if type_ == "offer" {
            info!("Received offer");
            trace!("Offer SDP: {}", sdp);
            info!("Offer SDP: {}", sdp);

            let ret = gst_sdp::SDPMessage::parse_buffer(sdp.as_bytes())
                .map_err(|_| anyhow!("Failed to parse SDP offer"))?;

            let (answer_promise, answer_future) = gst::Promise::new_future();
            let answer_promise = Arc::new(answer_promise);

            // And then asynchronously start our pipeline and do the next steps. The
            // pipeline needs to be started before we can create an answer
            let app_clone = self.downgrade();
            self.pipeline.call_async(move |_pipeline| {
                let app = upgrade_weak!(app_clone);

                let offer = gst_webrtc::WebRTCSessionDescription::new(
                    gst_webrtc::WebRTCSDPType::Offer,
                    ret,
                );

                app.0
                    .webrtcbin
                    .emit("set-remote-description", &[&offer, &None::<gst::Promise>])
                    .expect("GStreamer Error in set-remote-description");

                app.0.webrtcbin.connect("notify::ice-gathering-state", false, move |_values| {
                    app_clone.upgrade()
                        .ok_or("Unable to get app")
                        .unwrap()
                        .on_ice_gathering_state_change(Arc::clone(&answer_promise))
                        .unwrap();

                    None
                })
                    .expect("GStreamer Error in notify::ice-gather-state");
            });

            Ok(answer_future)
        } else {
            bail!("Sdp type is not \"offer\" but \"{}\"", type_)
        }
    }

    // // Handle incoming ICE candidates from the peer by passing them to webrtcbin
    // fn handle_ice(&self, sdp_mline_index: u32, candidate: &str) -> Result<(), anyhow::Error> {
    //     self.webrtcbin
    //         .emit("add-ice-candidate", &[&sdp_mline_index, &candidate])
    //         .unwrap();
    //
    //     Ok(())
    // }
    //
    // // Asynchronously send ICE candidates to the peer via the WebSocket connection as a JSON
    // // message
    // fn on_ice_candidate(&self, mlineindex: u32, candidate: String) -> Result<(), anyhow::Error> {
    //     let message = serde_json::to_string(&JsonMsg::Ice {
    //         candidate,
    //         sdp_mline_index: mlineindex,
    //     })
    //     .unwrap();
    //
    //     self.send_msg_tx
    //         .lock()
    //         .unwrap()
    //         .unbounded_send(WsMessage::Text(message))
    //         .with_context(|| format!("Failed to send ICE candidate"))?;
    //
    //     Ok(())
    // }

    // // Whenever there's a new incoming, encoded stream from the peer create a new decodebin
    // fn on_incoming_stream(&self, pad: &gst::Pad) -> Result<(), anyhow::Error> {
    //     // Early return for the source pads we're adding ourselves
    //     if pad.get_direction() != gst::PadDirection::Src {
    //         return Ok(());
    //     }
    //
    //     let decodebin = gst::ElementFactory::make("decodebin", None).unwrap();
    //     let app_clone = self.downgrade();
    //     decodebin.connect_pad_added(move |_decodebin, pad| {
    //         let app = upgrade_weak!(app_clone);
    //
    //         if let Err(err) = app.on_incoming_decodebin_stream(pad) {
    //             gst_element_error!(
    //                 app.pipeline,
    //                 gst::LibraryError::Failed,
    //                 ("Failed to handle decoded stream: {:?}", err)
    //             );
    //         }
    //     });
    //
    //     self.pipeline.add(&decodebin).unwrap();
    //     decodebin.sync_state_with_parent().unwrap();
    //
    //     let sinkpad = decodebin.get_static_pad("sink").unwrap();
    //     pad.link(&sinkpad).unwrap();
    //
    //     Ok(())
    // }

    // // Handle a newly decoded decodebin stream and depending on its type, create the relevant
    // // elements or simply ignore it
    // fn on_incoming_decodebin_stream(&self, pad: &gst::Pad) -> Result<(), anyhow::Error> {
    //     let caps = pad.get_current_caps().unwrap();
    //     let name = caps.get_structure(0).unwrap().get_name();
    //
    //     let sink = if name.starts_with("video/") {
    //         gst::parse_bin_from_description(
    //             "queue ! videoconvert ! videoscale ! autovideosink",
    //             true,
    //         )?
    //     } else if name.starts_with("audio/") {
    //         gst::parse_bin_from_description(
    //             "queue ! audioconvert ! audioresample ! autoaudiosink",
    //             true,
    //         )?
    //     } else {
    //         println!("Unknown pad {:?}, ignoring", pad);
    //         return Ok(());
    //     };
    //
    //     self.pipeline.add(&sink).unwrap();
    //     sink.sync_state_with_parent()
    //         .with_context(|| format!("can't start sink for stream {:?}", caps))?;
    //
    //     let sinkpad = sink.get_static_pad("sink").unwrap();
    //     pad.link(&sinkpad)
    //         .with_context(|| format!("can't link sink for stream {:?}", caps))?;
    //
    //     Ok(())
    // }
}

// Make sure to shut down the pipeline when it goes out of scope
// to release any system resources
impl Drop for AppInner {
    fn drop(&mut self) {
        let _ = self.pipeline.set_state(gst::State::Null);
    }
}

// Check if all GStreamer plugins we require are available
pub fn check_plugins() -> Result<(), anyhow::Error> {
    let needed = [
        "videotestsrc",
        "audiotestsrc",
        "videoconvert",
        "audioconvert",
        "autodetect",
        "opus",
        "vpx",
        "webrtc",
        "nice",
        "dtls",
        "srtp",
        "rtpmanager",
        "rtp",
        "playback",
        "videoscale",
        "audioresample",
    ];

    let registry = gst::Registry::get();
    let missing = needed
        .iter()
        .filter(|n| registry.find_plugin(n).is_none())
        .cloned()
        .collect::<Vec<_>>();

    if !missing.is_empty() {
        bail!("Missing plugins: {:?}", missing);
    } else {
        Ok(())
    }
}
