#![allow(non_camel_case_types, non_upper_case_globals, non_snake_case)]

use glib;
use glib::subclass;
use glib::subclass::prelude::*;
use gst;
use gst::prelude::*;
use gst::subclass::prelude::*;
use gst_audio;
use gst_base;
use gst_base::prelude::*;
use gst_base::subclass::prelude::*;

use std::sync::Mutex;
use std::{i32, u32};

use connect_ndi;
// use ndi_struct;
use ndisys::*;
use stop_ndi;

use hashmap_receivers;
use byte_slice_cast::AsMutSliceOf;

#[derive(Debug, Clone)]
struct Settings {
    stream_name: String,
    ip: String,
    loss_threshold: u32,
    id_receiver: i8,
    latency: Option<gst::ClockTime>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            stream_name: String::from("Fixed ndi stream name"),
            ip: String::from(""),
            loss_threshold: 5,
            id_receiver: 0,
            latency: None,
        }
    }
}

static PROPERTIES: [subclass::Property; 3] = [
subclass::Property("stream-name", |_| {
    glib::ParamSpec::string(
        "stream-name",
        "Sream Name",
        "Name of the streaming device",
        None,
        glib::ParamFlags::READWRITE,
    )
}),
subclass::Property("ip", |_| {
    glib::ParamSpec::string(
        "ip",
        "Stream IP",
        "IP of the streaming device. Ex: 127.0.0.1:5961",
        None,
        glib::ParamFlags::READWRITE,
    )
}),
subclass::Property("loss-threshold", |_| {
    glib::ParamSpec::uint(
        "loss-threshold",
        "Loss threshold",
        "Loss threshold",
        0,
        60,
        5,
        glib::ParamFlags::READWRITE,
    )
}),
];

struct State {
    info: Option<gst_audio::AudioInfo>,
}

impl Default for State {
    fn default() -> State {
        State { info: None }
    }
}

struct TimestampData {
    offset: u64,
}

struct NdiAudioSrc {
    cat: gst::DebugCategory,
    settings: Mutex<Settings>,
    state: Mutex<State>,
    timestamp_data: Mutex<TimestampData>,
}

impl ObjectSubclass for NdiAudioSrc {

    const NAME: &'static str = "NdiAudioSrc";
    type ParentType = gst_base::BaseSrc;
    type Instance = gst::subclass::ElementInstanceStruct<Self>;
    type Class = subclass::simple::ClassStruct<Self>;

    glib_object_subclass!();

    fn new() -> Self {
        Self {
            cat: gst::DebugCategory::new(
                "ndiaudiosrc",
                gst::DebugColorFlags::empty(),
                "NewTek NDI Audio Source",
            ),
            settings: Mutex::new(Default::default()),
            state: Mutex::new(Default::default()),
            timestamp_data: Mutex::new(TimestampData { offset: 0}),
        }
    }

    fn class_init(klass: &mut subclass::simple::ClassStruct<Self>) {
        klass.set_metadata(
            "NewTek NDI Audio Source",
            "Source",
            "NewTek NDI audio source",
            "Ruben Gonzalez <rubenrua@teltek.es>, Daniel Vilar <daniel.peiteado@teltek.es>",
        );

        let caps = gst::Caps::new_simple(
            "audio/x-raw",
            &[
            (
                "format",
                &gst::List::new(&[
                    //TODO add more formats?
                    //&gst_audio::AUDIO_FORMAT_F32.to_string(),
                    //&gst_audio::AUDIO_FORMAT_F64.to_string(),
                    &gst_audio::AUDIO_FORMAT_S16.to_string(),
                    ]),
                ),
                ("rate", &gst::IntRange::<i32>::new(1, i32::MAX)),
                ("channels", &gst::IntRange::<i32>::new(1, i32::MAX)),
                ("layout", &"interleaved"),
                ("channel-mask", &gst::Bitmask::new(0)),
                ],
            );

            let src_pad_template = gst::PadTemplate::new(
                "src",
                gst::PadDirection::Src,
                gst::PadPresence::Always,
                &caps,
            )
            .unwrap();
            klass.add_pad_template(src_pad_template);

            klass.install_properties(&PROPERTIES);
        }
    }

    impl ObjectImpl for NdiAudioSrc {
        glib_object_impl!();

        fn constructed(&self, obj: &glib::Object) {
            self.parent_constructed(obj);

            let basesrc = obj.downcast_ref::<gst_base::BaseSrc>().unwrap();
            // Initialize live-ness and notify the base class that
            // we'd like to operate in Time format
            basesrc.set_live(true);
            basesrc.set_format(gst::Format::Time);
            basesrc.set_do_timestamp(true);
        }

        fn set_property(&self, obj: &glib::Object, id: usize, value: &glib::Value) {
            let prop = &PROPERTIES[id];
            let basesrc = obj.downcast_ref::<gst_base::BaseSrc>().unwrap();

            match *prop {
                subclass::Property("stream-name", ..) => {
                    let mut settings = self.settings.lock().unwrap();
                    let stream_name = value.get().unwrap();
                    gst_debug!(
                        self.cat,
                        obj: basesrc,
                        "Changing stream-name from {} to {}",
                        settings.stream_name,
                        stream_name
                    );
                    settings.stream_name = stream_name;
                    drop(settings);
                }
                subclass::Property("ip", ..) => {
                    let mut settings = self.settings.lock().unwrap();
                    let ip = value.get().unwrap();
                    gst_debug!(
                        self.cat,
                        obj: basesrc,
                        "Changing ip from {} to {}",
                        settings.ip,
                        ip
                    );
                    settings.ip = ip;
                    drop(settings);
                }
                subclass::Property("loss-threshold", ..) => {
                    let mut settings = self.settings.lock().unwrap();
                    let loss_threshold = value.get().unwrap();
                    gst_debug!(
                        self.cat,
                        obj: basesrc,
                        "Changing loss threshold from {} to {}",
                        settings.loss_threshold,
                        loss_threshold
                    );
                    settings.loss_threshold = loss_threshold;
                    drop(settings);
                }
                _ => unimplemented!(),
            }
        }

        fn get_property(&self, _obj: &glib::Object, id: usize) -> Result<glib::Value, ()> {
            let prop = &PROPERTIES[id];

            match *prop {
                subclass::Property("stream-name", ..) => {
                    let settings = self.settings.lock().unwrap();
                    Ok(settings.stream_name.to_value())
                }
                subclass::Property("ip", ..) => {
                    let settings = self.settings.lock().unwrap();
                    Ok(settings.ip.to_value())
                }
                subclass::Property("loss-threshold", ..) => {
                    let settings = self.settings.lock().unwrap();
                    Ok(settings.loss_threshold.to_value())
                }
                _ => unimplemented!(),
            }
        }
    }

    impl ElementImpl for NdiAudioSrc {
        fn change_state(
            &self,
            element: &gst::Element,
            transition: gst::StateChange,
        ) ->  Result<gst::StateChangeSuccess, gst::StateChangeError> {
            if transition == gst::StateChange::PausedToPlaying {
                let mut receivers = hashmap_receivers.lock().unwrap();
                let settings = self.settings.lock().unwrap();

                let receiver = receivers.get_mut(&settings.id_receiver).unwrap();
                let recv = &receiver.ndi_instance;
                let pNDI_fs = recv.fs;

                let audio_frame: NDIlib_audio_frame_v2_t = Default::default();

                unsafe {
                    NDIlib_framesync_capture_audio(pNDI_fs, &audio_frame, 48000, 1, 1000);
                }
                gst_debug!(self.cat, obj: element, "NDI audio frame received: {:?}", audio_frame);

                unsafe {
                    NDIlib_framesync_free_audio(pNDI_fs, &audio_frame);
                }
            }
            self.parent_change_state(element, transition)
        }
    }

    impl BaseSrcImpl for NdiAudioSrc {
        fn set_caps(&self, element: &gst_base::BaseSrc, caps: &gst::CapsRef) -> Result<(), gst::LoggableError> {
            let info = match gst_audio::AudioInfo::from_caps(caps) {
                None => return Err(gst_loggable_error!(self.cat, "Failed to build `AudioInfo` from caps {}", caps)),
                Some(info) => info,
            };

            gst_debug!(self.cat, obj: element, "Configuring for caps {}", caps);

            let mut state = self.state.lock().unwrap();
            state.info = Some(info);

            Ok(())
        }

        fn start(&self, element: &gst_base::BaseSrc) -> Result<(), gst::ErrorMessage> {
            *self.state.lock().unwrap() = Default::default();

            let mut settings = self.settings.lock().unwrap();
            settings.id_receiver = connect_ndi(
                self.cat,
                element,
                &settings.ip.clone(),
                &settings.stream_name.clone(),
            );

            match settings.id_receiver {
                0 => Err(gst_error_msg!(
                gst::ResourceError::NotFound,
                ["Could not connect to this source"]
            )),
                _ => Ok(())
            }
        }

        fn stop(&self, element: &gst_base::BaseSrc) -> Result<(), gst::ErrorMessage> {
            *self.state.lock().unwrap() = Default::default();

            let settings = self.settings.lock().unwrap();
            stop_ndi(self.cat, element, settings.id_receiver);
            // Commented because when adding ndi destroy stopped in this line
            //*self.state.lock().unwrap() = Default::default();
            Ok(())
        }

        fn query(&self, element: &gst_base::BaseSrc, query: &mut gst::QueryRef) -> bool {
            use gst::QueryView;
            if let QueryView::Scheduling(ref mut q) = query.view_mut() {
                q.set(gst::SchedulingFlags::SEQUENTIAL, 1, -1, 0);
                q.add_scheduling_modes(&[gst::PadMode::Pull]);
                return true;
            }
            if let QueryView::Latency(ref mut q) = query.view_mut() {
                let settings = &*self.settings.lock().unwrap();
                let state = self.state.lock().unwrap();

                if let Some(ref _info) = state.info {
                    let latency = settings.latency.unwrap();
                    gst_debug!(self.cat, obj: element, "Returning latency {}", latency);
                    q.set(true, latency, gst::CLOCK_TIME_NONE);
                    return true;
                } else {
                    return false;
                }
            }
            BaseSrcImplExt::parent_query(self, element, query)
        }

        fn fixate(&self, element: &gst_base::BaseSrc, caps: gst::Caps) -> gst::Caps {
            let receivers = hashmap_receivers.lock().unwrap();
            let mut settings = self.settings.lock().unwrap();

            let receiver = receivers.get(&settings.id_receiver).unwrap();

            let recv = &receiver.ndi_instance;
            let pNDI_fs = recv.fs;

            let audio_frame: NDIlib_audio_frame_v2_t = Default::default();

            // TODO: Set sample rate and no_channels as a setting
            let no_samples = 1000 as u64;
            let sample_rate = 48000 as u64;
            let no_channels = 1 as u32;
            unsafe {
                NDIlib_framesync_capture_audio(pNDI_fs, &audio_frame, sample_rate as i32, no_channels as i32, no_samples as i32);
            }
            gst_log!(self.cat, obj: element, "Fixate: {:?}", (sample_rate as i32));

            settings.latency = gst::SECOND.mul_div_floor(no_samples, sample_rate);

            let mut caps = gst::Caps::truncate(caps);
            {
                let caps = caps.make_mut();
                let s = caps.get_mut_structure(0).unwrap();
                s.fixate_field_nearest_int("rate", sample_rate as i32);
                s.fixate_field_nearest_int("channels", no_channels as i32);
                s.fixate_field_str("layout", "interleaved");
                s.set_value("channel-mask", gst::Bitmask::new(gst_audio::AudioChannelPosition::get_fallback_mask(no_channels)).to_send_value());
            }

            let _ = element.post_message(&gst::Message::new_latency().src(Some(element)).build());
            unsafe {
                NDIlib_framesync_free_audio(pNDI_fs, &audio_frame);
            }

            self.parent_fixate(element, caps)
        }

        fn create(
            &self,
            element: &gst_base::BaseSrc,
            _offset: u64,
            _length: u32,
        ) -> Result<gst::Buffer, gst::FlowError> {
            let _settings = &*self.settings.lock().unwrap();

            let mut timestamp_data = self.timestamp_data.lock().unwrap();

            // Catch it in the settings or in the frame if different
            let no_samples = 1000 as u64;
            let sample_rate = 48000 as u64;
            let no_channels = 1 as u32;

            let state = self.state.lock().unwrap();
            let _info = match state.info {
                None => {
                    gst_element_error!(element, gst::CoreError::Negotiation, ["Have no caps yet"]);
                    return Err(gst::FlowError::NotNegotiated);
                }
                Some(ref info) => info.clone(),
            };
            let receivers = hashmap_receivers.lock().unwrap();

            let recv = &receivers.get(&_settings.id_receiver).unwrap().ndi_instance;
            let pNDI_fs = recv.fs;

            let audio_frame: NDIlib_audio_frame_v2_t = Default::default();

            unsafe {
                NDIlib_framesync_capture_audio(pNDI_fs, &audio_frame, sample_rate as i32, no_channels as i32, no_samples as i32);
            }
            gst_log!(self.cat, obj: element, "NDI audio frame received: {:?}", (audio_frame));

            // We multiply by 2 because is the size in bytes of an i16 variable
            let buff_size = (no_samples * no_channels as u64 * 2) as usize;

            let mut buffer = gst::Buffer::with_size(buff_size).unwrap();
            {
                let buffer = buffer.get_mut().unwrap();

                let pts_start: gst::ClockTime = timestamp_data.offset
                    .mul_div_floor(gst::SECOND_VAL, sample_rate)
                    .unwrap()
                    .into();
                timestamp_data.offset += no_samples;
                let pts_end: gst::ClockTime = timestamp_data.offset
                    .mul_div_floor(gst::SECOND_VAL, sample_rate)
                    .unwrap()
                    .into();
                buffer.set_pts(pts_start);
                buffer.set_duration(pts_end - pts_start);

                let mut dst: NDIlib_audio_frame_interleaved_16s_t = Default::default();
                dst.reference_level = 0;
                dst.p_data = buffer.map_writable().unwrap().as_mut_slice_of::<i16>().unwrap().as_mut_ptr();
                unsafe {
                    NDIlib_util_audio_to_interleaved_16s_v2(&audio_frame, &mut dst);
                    NDIlib_framesync_free_audio(pNDI_fs, &audio_frame);
                }
            }
            gst_log!(self.cat, obj: element, "Produced buffer {:?}", buffer);
            Ok(buffer)
        }
    }

    pub fn register(plugin: &gst::Plugin) -> Result<(), glib::BoolError> {
        gst::Element::register(plugin, "ndiaudiosrc", 0, NdiAudioSrc::get_type())
    }
