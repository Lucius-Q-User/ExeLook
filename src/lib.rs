#![feature(try_trait)]
use std::{
    ffi::{c_void, CStr},
    ptr,
    panic
};

mod icon_group;
mod dib;
mod exelook;

#[allow(non_upper_case_globals)]
const kCFStringEncodingUTF8: u32 = 0x0800_0100;
#[allow(non_upper_case_globals)]
const kCGRenderingIntentDefault: u32 = 0;
#[allow(non_upper_case_globals)]
const kCGImageAlphaLast:u32 = 3;
#[allow(non_upper_case_globals)]
const kCGBitmapByteOrder32Big:u32 = 4 << 12;

#[repr(C)]
pub struct CFUUID {
    _private: [u8; 0],
}
#[repr(C)]
pub struct CFString {
    _private: [u8; 0],
}
#[repr(C)]
pub struct CFData {
    _private: [u8; 0],
}
#[repr(C)]
pub struct CFURL {
    _private: [u8; 0],
}

#[repr(C)]
pub struct QLThumbnailRequest {
    _private: [u8; 0],
}
#[repr(C)]
pub struct CGImage {
    _private: [u8; 0],
}
#[repr(C)]
pub struct CGDataProvider {
    _private: [u8; 0],
}
#[repr(C)]
pub struct CGColorSpace {
    _private: [u8; 0],
}
#[allow(improper_ctypes)]
type DataReleaseCallback = unsafe extern fn(info: *mut Vec<u8>, data: *const c_void, size: usize);
#[link(name = "CoreFoundation", kind = "framework")]
#[link(name = "QuickLook", kind = "framework")]
#[link(name = "CoreServices", kind = "framework")]
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CFEqual(a: *const CFUUID, b: *const CFUUID) -> bool;
    fn CFUUIDCreateFromString(alloc: *const c_void, uuidStr: *const CFString) -> *const CFUUID;
    fn CFStringCreateWithCString(alloc: *const c_void, c_str: *const u8, encoding: u32) -> *const CFString;
    fn CFRelease(o: *const c_void);
    fn CFPlugInAddInstanceForFactory(o: *const CFUUID);
    fn CFPlugInRemoveInstanceForFactory(o: *const CFUUID);
    fn CFUUIDCreateFromUUIDBytes(alloc: *const c_void, uuid: REFIID) -> *const CFUUID;
    fn CFURLGetFileSystemRepresentation(url: *const CFURL, resolveAgainstBase: bool, buffer: *const u8, maxBufLen: isize) -> bool;
    fn QLThumbnailRequestSetImage(thumb: *const QLThumbnailRequest, image: *const CGImage, properties: *const c_void);
    fn CGImageCreateWithPNGDataProvider(provider: *const CGDataProvider, decode: *const c_void, interpolate: bool, intent: u32) -> *const CGImage;
    fn CGImageCreate(width: usize, height: usize, bpc: usize, bpp: usize, bpr: usize, colorspace: *const CGColorSpace, bitmap_info: u32, provider: *const CGDataProvider, decode: *const c_void, interpolate: bool, intent: u32) -> *const CGImage;
    #[allow(improper_ctypes)]
    fn CGDataProviderCreateWithData(info: *mut Vec<u8>, data: *const u8, size: usize, callback: DataReleaseCallback) -> *const CGDataProvider;
    fn CGDataProviderRelease(provider: *const CGDataProvider);
    fn CGImageRelease(image: *const CGImage);
    fn CGColorSpaceCreateDeviceRGB() -> *const CGColorSpace;
    fn CGColorSpaceRelease(space: *const CGColorSpace);
}

#[repr(C)]
struct CGSize {
    width: f64,
    height: f64
}

#[repr(C)]
struct REFIID {
    bytes: [u8; 16]
}

#[repr(C)]
struct QLGeneratorConduitItf {
    reserved: *const c_void,
    query_interface: unsafe extern fn(this: *mut QLGeneratorPlugin, iid: REFIID, ppv: *mut *mut QLGeneratorPlugin) -> u32,
    add_ref: unsafe extern fn(this: *mut QLGeneratorPlugin) -> u32,
    release: unsafe extern fn(this: *mut QLGeneratorPlugin) -> u32,
    generate_thumbnail_for_url: unsafe extern fn(this: *mut QLGeneratorPlugin, thumbnail: *mut QLThumbnailRequest, url: *const CFURL, contentTypeUTI: *const c_void, options: *const c_void, maxSize: CGSize) -> i32,
    cancel_thumbnail_generation: unsafe extern fn(this: *mut QLGeneratorPlugin, thumbnail: *const c_void),
    generate_preview_for_url: unsafe extern fn(this: *mut QLGeneratorPlugin, preview: *const c_void, url: *const c_void, contentTypeUTI: *const c_void, options: *const c_void) -> i32,
    cancel_preview_generation: unsafe extern fn(this: *mut QLGeneratorPlugin, preview: *const c_void),
}

#[repr(C)]
pub struct QLGeneratorPlugin {
    conduit_itf: *mut QLGeneratorConduitItf,
    factory_uuid: *const CFUUID,
    ref_count: u32,
}

extern "C" fn cancel_generation(_: *mut QLGeneratorPlugin, _: *const c_void) {
}

unsafe extern "C" fn release_data(info: *mut Vec<u8>, _: *const c_void, _: usize) {
    Box::from_raw(info);
}

unsafe extern "C" fn generate_thumbnail_for_url(_: *mut QLGeneratorPlugin, req: *mut QLThumbnailRequest, url: *const CFURL, _: *const c_void, _: *const c_void, _: CGSize) -> i32 {
    let path = [0; 1024];
    CFURLGetFileSystemRepresentation(url, false, path.as_ptr(), 1024);
    let path_str = CStr::from_ptr(path.as_ptr() as *const i8);
    let _ = panic::catch_unwind(|| {
        match exelook::exelook(path_str) {
            Ok((png_bytes, true, _, _)) => {
                let data = png_bytes.as_ptr();
                let size = png_bytes.len();
                let boxed = Box::new(png_bytes);
                let info = Box::into_raw(boxed);
                let provider = CGDataProviderCreateWithData(info, data, size, release_data);
                let image = CGImageCreateWithPNGDataProvider(provider, ptr::null(), false, kCGRenderingIntentDefault);
                CGDataProviderRelease(provider);
                QLThumbnailRequestSetImage(req, image, ptr::null());
                CGImageRelease(image);
            },
            Ok((raw_bytes, false, width, height)) => {
                let data = raw_bytes.as_ptr();
                let size = raw_bytes.len();
                let boxed = Box::new(raw_bytes);
                let info = Box::into_raw(boxed);
                let provider = CGDataProviderCreateWithData(info, data, size, release_data);
                let rgb = CGColorSpaceCreateDeviceRGB();
                let image = CGImageCreate(width as usize, height as usize, 8, 32, width as usize * 4, rgb,
                                          kCGImageAlphaLast | kCGBitmapByteOrder32Big,
                                          provider, ptr::null(), false, kCGRenderingIntentDefault);
                CGDataProviderRelease(provider);
                CGColorSpaceRelease(rgb);
                QLThumbnailRequestSetImage(req, image, ptr::null());
                CGImageRelease(image);
            },
            _ => {}
        }
    });
    0
}

extern "C" fn generate_preview_for_url(_: *mut QLGeneratorPlugin, _: *const c_void, _: *const c_void, _: *const c_void, _: *const c_void) -> i32 {
    0
}


unsafe extern "C" fn query_interface(this: *mut QLGeneratorPlugin, iid: REFIID, ppv: *mut *mut QLGeneratorPlugin) -> u32 {
    let requested_uid = CFUUIDCreateFromUUIDBytes(ptr::null(), iid);
    let my_uuid_str = CFStringCreateWithCString(ptr::null(), "865AF5E0-6D30-4345-951B-D37105754F2D\0".as_ptr(), kCFStringEncodingUTF8);
    let my_uuid = CFUUIDCreateFromString(ptr::null(), my_uuid_str);
    let result = if CFEqual(my_uuid, requested_uid) {
        *ppv = this;
        ((*(*this).conduit_itf).add_ref)(this);
        (*(*this).conduit_itf).cancel_preview_generation = cancel_generation;
        (*(*this).conduit_itf).cancel_thumbnail_generation = cancel_generation;
        (*(*this).conduit_itf).generate_thumbnail_for_url = generate_thumbnail_for_url;
        (*(*this).conduit_itf).generate_preview_for_url = generate_preview_for_url;
        0
    } else {
        *ppv = ptr::null_mut();
        0x8000_0004
    };
    CFRelease(requested_uid as *const c_void);
    CFRelease(my_uuid_str as *const c_void);
    CFRelease(my_uuid as *const c_void);
    result
}
unsafe extern "C" fn add_ref(this: *mut QLGeneratorPlugin) -> u32 {
    (*this).ref_count += 1;
    (*this).ref_count
}

unsafe extern "C" fn release(this: *mut QLGeneratorPlugin) -> u32 {
    (*this).ref_count -= 1;
    if (*this).ref_count == 0 {
        let fid = (*this).factory_uuid;
        CFPlugInRemoveInstanceForFactory(fid);
        CFRelease(fid as *const c_void);
        Box::from_raw((*this).conduit_itf);
        Box::from_raw(this);
        0
    } else {
        (*this).ref_count
    }
}


#[no_mangle]
pub unsafe extern fn quick_look_generator_plugin_factory(_: *const c_void, type_id: *const CFUUID) -> *const QLGeneratorPlugin {
    let ql_uuid_str = CFStringCreateWithCString(ptr::null(), "5E2D9680-5022-40FA-B806-43349622E5B9\0".as_ptr(), kCFStringEncodingUTF8);
    let ql_uuid = CFUUIDCreateFromString(ptr::null(), ql_uuid_str);
    let result = if CFEqual(ql_uuid, type_id) {
        let factory_uuid_str = CFStringCreateWithCString(ptr::null(),
            "9C10F405-F865-4819-9E96-9B783061FA75\0".as_ptr(), kCFStringEncodingUTF8);
        let factory_uuid = CFUUIDCreateFromString(ptr::null(), factory_uuid_str);
        let conduit_itf = Box::new(QLGeneratorConduitItf {
            query_interface, add_ref, release, generate_thumbnail_for_url, generate_preview_for_url,
            reserved: ptr::null(),
            cancel_preview_generation: cancel_generation,
            cancel_thumbnail_generation: cancel_generation
        });
        let this = Box::new(QLGeneratorPlugin {
            factory_uuid,
            ref_count: 1,
            conduit_itf: Box::into_raw(conduit_itf),
        });
        CFPlugInAddInstanceForFactory(factory_uuid);
        CFRelease(factory_uuid_str as *const c_void);
        Box::into_raw(this)
    } else {
        ptr::null()
    };
    CFRelease(ql_uuid_str as *const c_void);
    CFRelease(ql_uuid as *const c_void);
    result
}
