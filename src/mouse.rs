use crate::device::{find_device, Device, DeviceError, CONTROL_REPORT_ID};
use crate::display::DisplayInfo;
use std::mem::{size_of, MaybeUninit};
use winapi::ctypes::c_void;
use winapi::shared::windef::POINT;
use winapi::um::winuser::GetCursorPos;

const MOUSE_REPORT_ID: u8 = 0x03;
const MOUSE_REPORT_SIZE: u8 = size_of::<MouseReport>() as u8;
const MOUSE_COORDINATES_RANGE: u32 = 32_768;

pub struct MouseClick {
    button: Option<MouseButton>,
    x: u16,
    y: u16,
    wheel_position: u8,
    display_index: Option<usize>,
}

impl MouseClick {
    pub fn new() -> Self {
        MouseClick {
            button: None,
            x: 0,
            y: 0,
            wheel_position: 0,
            display_index: None,
        }
    }

    pub fn set_position(mut self, x: u16, y: u16) -> Self {
        self.x = x;
        self.y = y;
        self
    }

    pub fn set_button(mut self, button: MouseButton) -> Self {
        self.button = Some(button);
        self
    }

    pub fn set_wheel_position(mut self, wheel_position: u8) -> Self {
        self.wheel_position = wheel_position;
        self
    }

    pub fn set_display_index(mut self, display_index: usize) -> Self {
        self.display_index = Some(display_index);
        self
    }
}

pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[repr(C)]
#[derive(Debug)]
struct MouseReport {
    control_report_id: u8,
    report_length: u8,
    report_id: u8,
    button: u8,
    x: u16,
    y: u16,
    wheel_position: u8,
}

pub struct Mouse<'di> {
    device: Device,
    primary_display_info: &'di DisplayInfo,
    displays_info: &'di Vec<DisplayInfo>,
    mouse_x_coord_per_pixel: f64,
    mouse_y_coord_per_pixel: f64,
}

impl<'di> Mouse<'di> {
    pub fn init(displays_info: &'di Vec<DisplayInfo>) -> Result<Self, DeviceError> {
        let device = find_device()?;

        // By default we use primary display.
        let primary_display_info = displays_info.iter().find(|d| d.primary).unwrap();

        let total_width = displays_info.iter().map(|d| d.width as u32).sum::<u32>() as u32;
        // For simplicity, we assume that all monitors will be lined up and will have the same height.
        let total_height = displays_info.first().unwrap().height as u32;

        // Report mouse coordinates are from 0 to 32767 for both axes.
        // For correct movement we need to recalculate coordinates based on monitor position.
        let mouse_x_coord_per_pixel = MOUSE_COORDINATES_RANGE as f64 / total_width as f64;
        let mouse_y_coord_per_pixel = MOUSE_COORDINATES_RANGE as f64 / total_height as f64;

        Ok(Mouse {
            device,
            primary_display_info,
            displays_info,
            mouse_x_coord_per_pixel,
            mouse_y_coord_per_pixel,
        })
    }

    pub fn send_click(&self, click: MouseClick) -> bool {
        let button = match click.button {
            None => 0,
            Some(MouseButton::Left) => 1,
            Some(MouseButton::Right) => 2,
            Some(MouseButton::Middle) => 4,
        };

        let display_info = match click.display_index {
            Some(display_index) => self
                .displays_info
                .get(display_index)
                .unwrap_or_else(|| &self.primary_display_info),
            _ => &self.primary_display_info,
        };

        let offset_x = self
            .displays_info
            .iter()
            .filter(|d| d.position.right < display_info.position.right)
            .map(|d| d.width as u32)
            .sum::<u32>();

        let offset_y = self
            .displays_info
            .iter()
            .filter(|d| d.position.top < display_info.position.top)
            .map(|d| d.height as u32)
            .sum::<u32>();

        let x = ((click.x as u32 + offset_x) as f64 * self.mouse_x_coord_per_pixel) as u16;
        let y = ((click.y as u32 + offset_y) as f64 * self.mouse_y_coord_per_pixel) as u16;

        let mut report = MouseReport {
            control_report_id: CONTROL_REPORT_ID,
            report_length: MOUSE_REPORT_SIZE,
            report_id: MOUSE_REPORT_ID,
            button,
            x,
            y,
            wheel_position: click.wheel_position,
        };

        self.device
            .send_report(&mut report as *mut _ as *mut c_void)
    }
}

#[derive(Debug)]
pub struct CursorPosition {
    pub x: u32,
    pub y: u32,
}

pub fn get_cursor_position() -> CursorPosition {
    let mut maybe_point = MaybeUninit::<POINT>::uninit();

    unsafe {
        GetCursorPos(maybe_point.as_mut_ptr());
    }

    let point = unsafe { maybe_point.assume_init() };

    let x = point.x as u32;
    let y = point.y as u32;

    CursorPosition { x, y }
}
