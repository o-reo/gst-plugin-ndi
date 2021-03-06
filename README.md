GStreamer NDI Plugin for Linux
====================

*Compiled and tested with Ubuntu 16.04.5, GStreamer 1.8.3 and NDI SDK 3.0.9 and 3.5.1*

This is a plugin for the [GStreamer](https://gstreamer.freedesktop.org/) multimedia framework that allows GStreamer to receive a stream from a [NDI](https://www.newtek.com/ndi/) source. This plugin has been developed by [Teltek](http://teltek.es/) and was funded by the [University of the Arts London](https://www.arts.ac.uk/) and [The University of Manchester](https://www.manchester.ac.uk/).

Currently the plugin has two source elements, `ndivideosrc` to get video from the stream and `ndiaudiosrc` for audio. By just providing the name or the ip of the stream, all the information required from the stream is picked up automatically, such as resolution, framerate, audio channels, ...

Some examples of how to use these elements from the command line:

```
#Information about the elements
gst-inspect-1.0 ndi
gst-inspect-1.0 ndivideosrc
gst-inspect-1.0 ndiaudiosrc

#Video pipeline
gst-launch-1.0 ndivideosrc stream-name="GC-DEV2 (OBS)" ! autovideosink
#Audio pipeline
gst-launch-1.0 ndiaudiosrc stream-name="GC-DEV2 (OBS)" ! autoaudiosink

#Video and audio pipeline
gst-launch-1.0 ndivideosrc stream-name="GC-DEV2 (OBS)" ! autovideosink ndiaudiosrc stream-name="GC-DEV2 (OBS)" ! autoaudiosink
```

Feel free to contribute to this project. Some ways you can contribute are:
* Testing with more hardware and software and reporting bugs
* Doing pull requests.

Compilation of the NDI element
-------
To compile the NDI element it's necessary to install Rust, the NDI SDK and the following packages for gstreamer:

```
apt-get install libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev \
      gstreamer1.0-plugins-base gstreamer1.0-plugins-good \
      gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly \
      gstreamer1.0-libav libgstrtspserver-1.0-dev

```
To install the required NDI library there are two options:
1. Download NDI SDK from NDI website and move the library to the correct location.
2. Use a [deb package](https://github.com/Palakis/obs-ndi/releases/download/4.5.2/libndi3_3.5.1-1_amd64.deb) made by the community. Thanks to [NDI plugin for OBS](https://github.com/Palakis/obs-ndi).

To install Rust, you can follow their documentation: https://www.rust-lang.org/en-US/install.html

Once all requirements are met, you can build the plugin by executing the following command from the project root folder:

```
cargo build
export GST_PLUGIN_PATH=`pwd`/target/debug
gst-inspect-1.0 ndi
```

If all went ok, you should see info related to the NDI element. To make the plugin available without using `GST_PLUGIN_PATH` it's necessary to copy the plugin to the gstreamer plugins folder.
```
cargo build --release
sudo install -o root -g root -m 644 target/release/libgstndi.so /usr/lib/x86_64-linux-gnu/gstreamer-1.0/
sudo ldconfig
gst-inspect-1.0 ndi
```

More info about GStreamer plugins written in Rust:
----------------------------------
https://github.com/sdroege/gstreamer-rs
https://github.com/sdroege/gst-plugin-rs

https://coaxion.net/blog/2018/01/how-to-write-gstreamer-elements-in-rust-part-1-a-video-filter-for-converting-rgb-to-grayscale/
https://coaxion.net/blog/2018/02/how-to-write-gstreamer-elements-in-rust-part-2-a-raw-audio-sine-wave-source/


License
-------
This plugin is licensed under the LGPL - see the ([LICENSE](LICENSE) file for details


Acknowledgments
-------
* University of the Arts London and The University of Manchester.
* Sebastian Dröge (@sdroege).
