#[cfg(feature = "real_ffmpeg")]
use ffmpeg_next as ffmpeg;
use lunaris_api::{
    render::{PixelFormat, RawImage},
    util::error::{LunarisError, Result},
};
use std::path::Path;

#[cfg(feature = "real_ffmpeg")]
pub struct Decoder {
    input: ffmpeg::format::context::Input,
    decoder: ffmpeg::decoder::Video,
    stream_index: usize,
    scaler: ffmpeg::software::scaling::Context,
    width: u32,
    height: u32,
}

#[cfg(not(feature = "real_ffmpeg"))]
pub struct Decoder {
    width: u32,
    height: u32,
}

// Send is needed because we move Decoder between threads (Orchestrator workers)
unsafe impl Send for Decoder {}

impl Decoder {
    pub fn new(path: &Path) -> Result<Self> {
        #[cfg(feature = "real_ffmpeg")]
        {
            let input = ffmpeg::format::input(&path).map_err(|e| LunarisError::Generic {
                reason: format!("Failed to open video file: {}", e),
            })?;

            let stream = input
                .streams()
                .best(ffmpeg::media::Type::Video)
                .ok_or(LunarisError::Generic {
                    reason: "No video stream found".to_string(),
                })?;

            let stream_index = stream.index();
            let context_decoder =
                ffmpeg::codec::context::Context::from_parameters(stream.parameters()).map_err(|e| {
                    LunarisError::Generic {
                        reason: format!("Failed to create codec context: {}", e),
                    }
                })?;

            let decoder = context_decoder
                .decoder()
                .video()
                .map_err(|e| LunarisError::Generic {
                    reason: format!("Failed to create video decoder: {}", e),
                })?;

            let width = decoder.width();
            let height = decoder.height();

            let scaler = ffmpeg::software::scaling::Context::get(
                decoder.format(),
                width,
                height,
                ffmpeg::format::Pixel::RGBA,
                width,
                height,
                ffmpeg::software::scaling::flag::BILINEAR,
            )
            .map_err(|e| LunarisError::Generic {
                reason: format!("Failed to create scaler: {}", e),
            })?;

            Ok(Self {
                input,
                decoder,
                stream_index,
                scaler,
                width,
                height,
            })
        }
        #[cfg(not(feature = "real_ffmpeg"))]
        {
            // Mock implementation
            Ok(Self {
                width: 1920,
                height: 1080,
            })
        }
    }

    pub fn decode_frame(&mut self, timestamp_ms: i64) -> Result<RawImage> {
        #[cfg(feature = "real_ffmpeg")]
        {
            // Seek to timestamp
            // Note: This is a simplified seek. Precise seeking is harder.
            let time_base = self.input.stream(self.stream_index).unwrap().time_base();
            let seek_ts = (timestamp_ms as f64 / 1000.0 / f64::from(time_base)).round() as i64;

            self.input
                .seek(seek_ts, ..seek_ts)
                .map_err(|e| LunarisError::Generic {
                    reason: format!("Seek failed: {}", e),
                })?;

            // Decode loop
            let mut decoded = ffmpeg::util::frame::Video::empty();
            for (stream, packet) in self.input.packets() {
                if stream.index() == self.stream_index {
                    self.decoder.send_packet(&packet).map_err(|e| LunarisError::Generic {
                        reason: format!("Packet send failed: {}", e),
                    })?;
                    
                    if self.decoder.receive_frame(&mut decoded).is_ok() {
                        // We got a frame. Scale it to RGBA.
                        let mut rgb_frame = ffmpeg::util::frame::Video::empty();
                        self.scaler.run(&decoded, &mut rgb_frame).map_err(|e| LunarisError::Generic {
                            reason: format!("Scaling failed: {}", e),
                        })?;

                        let data = rgb_frame.data(0);
                        let stride = rgb_frame.stride(0);
                        
                        // Copy data tightly packed
                        let mut bytes = Vec::with_capacity((self.width * self.height * 4) as usize);
                        for y in 0..self.height {
                            let start = (y as usize) * stride;
                            let end = start + (self.width as usize) * 4;
                            bytes.extend_from_slice(&data[start..end]);
                        }

                        return RawImage::from_bytes(
                            PixelFormat::Rgba8Unorm,
                            self.width,
                            self.height,
                            bytes,
                        );
                    }
                }
            }

            Err(LunarisError::Generic {
                reason: "End of stream or decode error".to_string(),
            })
        }
        #[cfg(not(feature = "real_ffmpeg"))]
        {
            // Return a dummy frame (checkerboard or solid color)
            // Changing color based on timestamp to simulate playback
            let r = (timestamp_ms % 255) as u8;
            let g = ((timestamp_ms / 2) % 255) as u8;
            let b = ((timestamp_ms / 3) % 255) as u8;
            
            let mut data = Vec::with_capacity((self.width * self.height * 4) as usize);
            for _ in 0..(self.width * self.height) {
                data.push(r);
                data.push(g);
                data.push(b);
                data.push(255);
            }
            
            RawImage::from_bytes(
                PixelFormat::Rgba8Unorm,
                self.width,
                self.height,
                data,
            )
        }
    }
}
