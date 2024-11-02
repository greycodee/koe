use libc::sleep;
use serde::Serialize;
use silk_v3_sys::{
    SKP_SILK_SDK_DecControlStruct, SKP_Silk_SDK_Decode, SKP_Silk_SDK_Get_Decoder_Size,
    SKP_Silk_SDK_InitDecoder, SKP_Silk_SDK_get_version, SKP_Silk_SDK_search_for_LBRR,
};
use std::fs::File;
use std::io::{Read, Seek, Write};
use std::sync::mpsc::Sender;
use std::{io, thread};
use tauri::ipc::Channel;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "event", content = "data")]
pub(crate) enum DownloadEvent<'a> {
    #[serde(rename_all = "camelCase")]
    Started {
        url: &'a str,
        download_id: usize,
        content_length: usize,
    },
    #[serde(rename_all = "camelCase")]
    Progress { val: f64 },
    #[serde(rename_all = "camelCase")]
    Finished { download_id: usize },
}

const MAX_BYTES_PER_FRAME: usize = 1024;
const MAX_INPUT_FRAMES: usize = 5;
// const MAX_FRAME_LENGTH: usize = 480;
const FRAME_LENGTH_MS: usize = 20;
const MAX_API_FS_KHZ: usize = 48;
const MAX_LBRR_DELAY: usize = 2;

pub fn silk_v3_decoder(
    in_file: &str,
    out_file: &str,
    on_event: Channel<DownloadEvent>,
) -> Result<(), io::Error> {
    let mut tottime: u64 = 0;
    let mut _tot_packets: i32 = 0;
    let mut payload = vec![0u8; MAX_BYTES_PER_FRAME * MAX_INPUT_FRAMES * (MAX_LBRR_DELAY + 1)];
    let mut fecpayload = vec![0u8; MAX_BYTES_PER_FRAME * MAX_INPUT_FRAMES];
    let mut n_bytes_per_packet = vec![0i16; MAX_LBRR_DELAY + 1];
    let mut out = vec![0i16; ((FRAME_LENGTH_MS * MAX_API_FS_KHZ) << 1) * MAX_INPUT_FRAMES];
    let mut dec_control = SKP_SILK_SDK_DecControlStruct {
        API_sampleRate: 24000,
        frameSize: 0,
        framesPerPacket: 1,
        moreInternalDecoderFrames: 0,
        inBandFECOffset: 0,
    };

    let mut bit_in_file = File::open(in_file)?;
    let mut speech_out_file = File::create(out_file)?;

    let mut header_buf = vec![0u8; 50];
    bit_in_file.read_exact(&mut header_buf[..1])?;
    if header_buf[0] != 0x02 {
        bit_in_file.read_exact(&mut header_buf[..8])?;
        if &header_buf[..8] != b"!SILK_V3" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Error: Wrong Header",
            ));
        }
    } else {
        bit_in_file.read_exact(&mut header_buf[..9])?;
        if &header_buf[..9] != b"#!SILK_V3" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Error: Wrong Header",
            ));
        }
    }

    let mut dec_size_bytes: i32 = 0;
    unsafe {
        if SKP_Silk_SDK_Get_Decoder_Size(&mut dec_size_bytes) != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SKP_Silk_SDK_Get_Decoder_Size failed",
            ));
        }
    }

    let ps_dec = unsafe { libc::malloc(dec_size_bytes as usize) };
    if ps_dec.is_null() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Failed to allocate decoder",
        ));
    }

    unsafe {
        if SKP_Silk_SDK_InitDecoder(ps_dec) != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SKP_Silk_SDK_InitDecoder failed",
            ));
        }
    }

    let mut payload_end = 0;
    for i in 0..MAX_LBRR_DELAY {
        let mut n_bytes: i16 = 0;
        bit_in_file.read_exact(unsafe {
            std::slice::from_raw_parts_mut(&mut n_bytes as *mut _ as *mut u8, 2)
        })?;
        bit_in_file.read_exact(&mut payload[payload_end..payload_end + n_bytes as usize])?;
        n_bytes_per_packet[i] = n_bytes;
        payload_end += n_bytes as usize;
        _tot_packets += 1;
    }

    // let mut tot_len = 0;
    let total_size = bit_in_file.metadata()?.len();
    #[allow(unused_assignments)]
    let mut processed_size = bit_in_file.stream_position()?;

    loop {
        let mut n_bytes: i16 = 0;
        if bit_in_file
            .read_exact(unsafe {
                std::slice::from_raw_parts_mut(&mut n_bytes as *mut _ as *mut u8, 2)
            })
            .is_err()
        {
            break;
        }
        if n_bytes < 0 {
            break;
        }
        if bit_in_file
            .read_exact(&mut payload[payload_end..payload_end + n_bytes as usize])
            .is_err()
        {
            break;
        }

        let mut _lost = 0;
        let mut payload_to_dec = &payload[..];
        unsafe {
            if ((libc::rand() >> 16) + (1 << 15)) as f32 / 65535.0 >= 0.0 {
                n_bytes_per_packet[MAX_LBRR_DELAY] = n_bytes;
                payload_end += n_bytes as usize;
            } else {
                n_bytes_per_packet[MAX_LBRR_DELAY] = 0;
            }
        }

        if n_bytes_per_packet[0] == 0 {
            _lost = 1;
            let mut payload_ptr = &payload[..];
            for i in 0..MAX_LBRR_DELAY {
                if n_bytes_per_packet[i + 1] > 0 {
                    let mut n_bytes_fec: i16 = 0;
                    unsafe {
                        SKP_Silk_SDK_search_for_LBRR(
                            payload_ptr.as_ptr(),
                            n_bytes_per_packet[i + 1] as i32,
                            (i + 1) as i32,
                            fecpayload.as_mut_ptr(),
                            &mut n_bytes_fec,
                        );
                    }
                    if n_bytes_fec > 0 {
                        payload_to_dec = &fecpayload[..];
                        n_bytes = n_bytes_fec;
                        _lost = 0;
                        break;
                    }
                }
                payload_ptr = &payload_ptr[n_bytes_per_packet[i + 1] as usize..];
            }
        } else {
            _lost = 0;
            n_bytes = n_bytes_per_packet[0];
            payload_to_dec = &payload[..];
        }

        let mut out_ptr = &mut out[..];
        let mut tot_len = 0;
        let start_time = std::time::Instant::now();

        if _lost == 0 {
            let mut frames = 0;
            loop {
                let mut len: i16 = 0;
                unsafe {
                    if SKP_Silk_SDK_Decode(
                        ps_dec,
                        &mut dec_control,
                        0,
                        payload_to_dec.as_ptr(),
                        n_bytes as i32,
                        out_ptr.as_mut_ptr(),
                        &mut len,
                    ) != 0
                    {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "SKP_Silk_SDK_Decode failed",
                        ));
                    }
                }
                frames += 1;
                out_ptr = &mut out_ptr[len as usize..];
                tot_len += len as usize;
                if frames > MAX_INPUT_FRAMES {
                    out_ptr = &mut out[..];
                    tot_len = 0;
                    frames = 0;
                }
                if dec_control.moreInternalDecoderFrames == 0 {
                    break;
                }
            }
        } else {
            for _ in 0..dec_control.framesPerPacket {
                let mut len: i16 = 0;
                unsafe {
                    if SKP_Silk_SDK_Decode(
                        ps_dec,
                        &mut dec_control,
                        1,
                        payload_to_dec.as_ptr(),
                        n_bytes as i32,
                        out_ptr.as_mut_ptr(),
                        &mut len,
                    ) != 0
                    {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "SKP_Silk_SDK_Decode failed",
                        ));
                    }
                }
                out_ptr = &mut out_ptr[len as usize..];
                tot_len += len as usize;
            }
        }

        tottime += start_time.elapsed().as_micros() as u64;
        _tot_packets += 1;

        speech_out_file.write_all(unsafe {
            std::slice::from_raw_parts(out.as_ptr() as *const u8, tot_len * 2)
        })?;

        let mut _tot_bytes = 0;
        for i in 0..MAX_LBRR_DELAY {
            _tot_bytes += n_bytes_per_packet[i + 1] as usize;
        }
        payload.copy_within(n_bytes_per_packet[0] as usize..payload_end, 0);
        payload_end -= n_bytes_per_packet[0] as usize;
        n_bytes_per_packet.copy_within(1.., 0);

        processed_size = bit_in_file.stream_position()?;
        let progress = (processed_size as f64 / total_size as f64) * 100.0;
        // if  tx.send(progress).is_ok() {
        //     println!("Progress: {:.2}% ", progress);
        // }

        on_event
            .send(DownloadEvent::Progress { val: progress })
            .unwrap();
        // println!("Progress: {:.2}% ", progress);
    }

    unsafe {
        libc::free(ps_dec);
    }

    println!("Decoding Finished: {} ms", tottime / 1000);
    Ok(())
}

pub fn get_version() -> Result<String, std::str::Utf8Error> {
    unsafe {
        let result = SKP_Silk_SDK_get_version();
        let c_str = std::ffi::CStr::from_ptr(result);
        let str_slice = c_str.to_str()?;
        Ok(str_slice.to_string())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    // #[test]
    // fn test_silk_v3_decoder() {
    //     let res = silk_v3_decoder(
    //         "/tmp/msg_152059061922b0890a24269102.amr",
    //         "/tmp/msg_152059061922b0890a24269102.pcm",
    //     );
    //     match res {
    //         Ok(_) => {
    //             println!("Decoding success!");
    //         },
    //         Err(e) => {
    //             panic!("ERR: {}",e);
    //         }
    //     }
    //     // assert_eq!(res.is_ok(), true);
    //     // assert_eq!(res, 0);
    // }

    #[test]
    fn test_get_version() {
        let version = get_version().unwrap();
        println!("Version: {}", version);
    }
}
