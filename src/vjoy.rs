use crate::axis::Axis;
use crate::button::{Button, ButtonState};
use crate::device::Device;
use crate::error::{AppError, Error, FFIError};
use crate::hat::HatState;
use crate::{FourWayHat, Hat};
use log::trace;
use vjoy_sys::{VjdStat, AXES_DISPLAY_NAMES, AXES_HID_USAGE, JOYSTICK_POSITION};

/// Main entry for this crate and controller for all vJoy devices.
///
/// [from_default_dll_location](Self::from_default_dll_location()) loads the vJoyInterface.dll from "C:/Program Files/vJoy/x64/vJoyInterface.dll".
/// Alternatively, you can provide a custom location via [from_dll_location](Self::from_dll_location()).
///
/// [get_device_state](Self::get_device_state()) returns the current state for a specific device.
/// You can alter this state and upload it to the virtual device via [update_device_state](Self::update_device_state()).
///
/// Note on IDs: Since the display names for the virtual devices and components are hardcoded into the .dll,
/// all IDs are one-based to avoid confusion.
/// ID-Ranges:
/// - Devices: 1..=16
/// - Buttons: 1..=128
/// - Axes: 1..=8
/// - Hat switches: 1..=4
/// ```no_run
/// # use vjoy::{VJoy, ButtonState, Error};///
/// let mut vjoy = VJoy::from_default_dll_location()?;
/// let mut device_1 = vjoy.get_device_state(1)?;
///
/// device_1.set_button(1, ButtonState::Pressed)?;
/// device_1.set_axis(1, i32::MAX)?;
///
/// vjoy.update_device_state(&device_1)?;
///
/// device_1.reset_all();
///
/// vjoy.update_device_state(&device_1)?;
/// # Ok::<(), Error>(())
/// ```
pub struct VJoy {
    ffi: vjoy_sys::vJoyInterface,
    devices: Vec<Device>,
}

impl VJoy {
    #[profiling::function]
    pub fn from_default_dll_location() -> Result<Self, Error> {
        Self::from_dll_location("C:/Program Files/vJoy/x64/vJoyInterface.dll")
    }

    #[profiling::function]
    pub fn from_dll_location(path: &str) -> Result<Self, Error> {
        let mut vjoy = Self::new(path)?;
        vjoy.fetch_devices();

        Ok(vjoy)
    }

    #[profiling::function]
    pub fn devices_cloned(&self) -> Vec<Device> {
        self.devices.clone()
    }

    #[profiling::function]
    pub fn devices(&self) -> std::slice::Iter<Device> {
        self.devices.iter()
    }

    #[profiling::function]
    pub fn devices_mut(&mut self) -> std::slice::IterMut<Device> {
        self.devices.iter_mut()
    }

    #[profiling::function]
    pub fn get_device_state(&self, device_id: u32) -> Result<Device, Error> {
        if device_id == 0 {
            return Err(Error::App(AppError::DeviceNotFound(device_id)));
        }

        match self.devices.get((device_id - 1) as usize) {
            Some(device) => Ok(device.clone()),
            None => Err(Error::App(AppError::DeviceNotFound(device_id))),
        }
    }

    #[profiling::function]
    pub fn get_device_state_ref(&self, device_id: u32) -> Result<&Device, Error> {
        if device_id == 0 {
            return Err(Error::App(AppError::DeviceNotFound(device_id)));
        }

        match self.devices.get((device_id - 1) as usize) {
            Some(device) => Ok(device),
            None => Err(Error::App(AppError::DeviceNotFound(device_id))),
        }
    }

    #[profiling::function]
    pub fn get_device_state_mut(&mut self, device_id: u32) -> Result<&mut Device, Error> {
        if device_id == 0 {
            return Err(Error::App(AppError::DeviceNotFound(device_id)));
        }

        match self.devices.get_mut((device_id - 1) as usize) {
            Some(device) => Ok(device),
            None => Err(Error::App(AppError::DeviceNotFound(device_id))),
        }
    }

    #[profiling::function]
    pub fn update_all_devices(&mut self) -> Result<(), Error> {
        for device in self.devices.iter() {
            // Axes value or default mid-point
            let axis_data: Vec<i32> = (0..8)
                .map(|index| {
                    if let Some(axis) = device.axes.get(index) {
                        axis.get()
                    } else {
                        16384
                    }
                })
                .collect();

            let button_data: Vec<ButtonState> = (0..128)
                .map(|index| {
                    if let Some(button) = device.buttons.get(index) {
                        button.get()
                    } else {
                        ButtonState::Released
                    }
                })
                .collect();

            // 4 fields á 32 buttons as single bits
            let mut button_field_data = [0; 4];
            for (i, field) in button_field_data.iter_mut().enumerate() {
                let start = i * 32;
                let end = i * 32 + 32;

                let buttons = &button_data[start..end];
                for (bit, state) in buttons.iter().enumerate().take(32) {
                    if *state == ButtonState::Pressed {
                        *field |= 0x1 << bit;
                    }
                }
            }

            let hats_data: Vec<u32> = (0..4)
                .map(|index| {
                    if let Some(hat) = device.hats.get(index) {
                        match hat.get() {
                            HatState::Continuous(c) => c,
                            HatState::Discrete(d) => d as u32,
                        }
                    } else {
                        0
                    }
                })
                .collect();

            let mut data = JOYSTICK_POSITION {
                bDevice: device.id as u8,

                wAxisX: axis_data[0],
                wAxisY: axis_data[1],
                wAxisZ: axis_data[2],
                wAxisXRot: axis_data[3],
                wAxisYRot: axis_data[4],
                wAxisZRot: axis_data[5],
                wDial: axis_data[6],
                wSlider: axis_data[7],

                wWheel: 0,
                wAileron: 0,
                wRudder: 0,
                wThrottle: 0,
                wAxisVX: 0,
                wAxisVY: 0,
                wAxisVZ: 0,
                wAxisVBRX: 0,
                wAxisVBRY: 0,
                wAxisVBRZ: 0,

                lButtons: button_field_data[0],
                lButtonsEx1: button_field_data[1],
                lButtonsEx2: button_field_data[2],
                lButtonsEx3: button_field_data[3],

                bHats: hats_data[0],
                bHatsEx1: hats_data[1],
                bHatsEx2: hats_data[2],
                bHatsEx3: hats_data[3],
            };

            Self::update_device_data(&self.ffi, device.id, &mut data)?;
        }

        Ok(())
    }

    #[profiling::function]
    pub fn update_device_state(&mut self, new_device_state: &Device) -> Result<(), Error> {
        if new_device_state.id == 0 {
            return Err(Error::App(AppError::DeviceNotFound(new_device_state.id)));
        }

        let device = match self.devices.get_mut((new_device_state.id - 1) as usize) {
            Some(device) => device,
            None => return Err(Error::App(AppError::DeviceNotFound(new_device_state.id))),
        };

        *device = new_device_state.clone();

        // Axes value or default mid-point
        let axis_data: Vec<i32> = (0..8)
            .map(|index| {
                if let Some(axis) = device.axes.get(index) {
                    axis.get()
                } else {
                    16384
                }
            })
            .collect();

        let button_data: Vec<ButtonState> = (0..128)
            .map(|index| {
                if let Some(button) = device.buttons.get(index) {
                    button.get()
                } else {
                    ButtonState::Released
                }
            })
            .collect();

        // 4 fields á 32 buttons as single bits
        let mut button_field_data = [0; 4];
        for (i, field) in button_field_data.iter_mut().enumerate() {
            let start = i * 32;
            let end = i * 32 + 32;

            let buttons = &button_data[start..end];
            for (bit, state) in buttons.iter().enumerate().take(32) {
                if *state == ButtonState::Pressed {
                    *field |= 0x1 << bit;
                }
            }
        }

        let hats_data: Vec<u32> = (0..4)
            .map(|index| {
                if let Some(hat) = device.hats.get(index) {
                    match hat.get() {
                        HatState::Continuous(c) => c,
                        HatState::Discrete(d) => d as u32,
                    }
                } else {
                    0
                }
            })
            .collect();

        let mut data = JOYSTICK_POSITION {
            bDevice: device.id as u8,

            wAxisX: axis_data[0],
            wAxisY: axis_data[1],
            wAxisZ: axis_data[2],
            wAxisXRot: axis_data[3],
            wAxisYRot: axis_data[4],
            wAxisZRot: axis_data[5],
            wDial: axis_data[6],
            wSlider: axis_data[7],

            wWheel: 0,
            wAileron: 0,
            wRudder: 0,
            wThrottle: 0,
            wAxisVX: 0,
            wAxisVY: 0,
            wAxisVZ: 0,
            wAxisVBRX: 0,
            wAxisVBRY: 0,
            wAxisVBRZ: 0,

            lButtons: button_field_data[0],
            lButtonsEx1: button_field_data[1],
            lButtonsEx2: button_field_data[2],
            lButtonsEx3: button_field_data[3],

            bHats: hats_data[0],
            bHatsEx1: hats_data[1],
            bHatsEx2: hats_data[2],
            bHatsEx3: hats_data[3],
        };

        Self::update_device_data(&self.ffi, device.id, &mut data)?;

        Ok(())
    }

    /// All vJoy devices share the same guid and vendor/device information.
    ///
    /// To differentiate between vJoy devices from other libraries (e.g. SDL2), you may use the configuration instead.
    #[profiling::function]
    pub fn get_id_for_configuration(
        &self,
        num_buttons: u32,
        num_axes: u32,
        num_hats: u32,
    ) -> Result<u32, Error> {
        let find: Vec<&Device> = self
            .devices
            .iter()
            .filter(|device| {
                device.buttons.len() as u32 == num_buttons
                    && device.axes.len() as u32 == num_axes
                    && device.hats.len() as u32 == num_hats
            })
            .collect();

        if find.len() > 1 {
            return Err(Error::App(AppError::DeviceConfigMultipleFound(
                num_buttons,
                num_axes,
                num_hats,
            )));
        }

        match find.first() {
            Some(device) => Ok(device.id),
            None => Err(Error::App(AppError::DeviceConfigNotFound(
                num_buttons,
                num_axes,
                num_hats,
            ))),
        }
    }

    #[profiling::function]
    fn new(path: &str) -> Result<Self, Error> {
        unsafe {
            let Ok(ffi) = vjoy_sys::vJoyInterface::new(path)
            else {
                return Err(Error::Ffi(FFIError::DynamicLybraryNotFound(path.to_string())));
            };

            Ok(Self {
                ffi,
                devices: Vec::new(),
            })
        }
    }

    #[profiling::function]
    fn fetch_devices(&mut self) {
        for device_id in 1..=16 {
            if self.acquire_device(device_id).is_ok() {
                let button_count = unsafe { self.ffi.GetVJDButtonNumber(device_id) } as u32;
                let buttons: Vec<Button> = (1..=button_count)
                    .map(|button_id| Button {
                        id: button_id as u8,
                        state: ButtonState::Released,
                    })
                    .collect();
                trace!("Device {} button count: {}", device_id, buttons.len());

                let mut axes = Vec::new();
                for axis_id in 1..=8 {
                    let axis_index = (axis_id - 1) as usize;
                    let axis_display_name = AXES_DISPLAY_NAMES[axis_index].to_string();
                    let axis_hid_usage = AXES_HID_USAGE[axis_index];
                    let exists = unsafe { self.ffi.GetVJDAxisExist(device_id, axis_hid_usage) };
                    trace!(
                        "Device {} axis id: {} display name: {} hid usage: {}",
                        device_id,
                        axis_id,
                        axis_display_name,
                        axis_hid_usage
                    );
                    if exists == 1 {
                        let axis = Axis {
                            display_name: axis_display_name,
                            hid_usage: axis_hid_usage,
                            id: axis_id,
                            value: 0,
                        };
                        axes.push(axis);
                    }
                }

                let hat_disc_count = unsafe { self.ffi.GetVJDDiscPovNumber(device_id) } as u32;
                let hat_cont_count = unsafe { self.ffi.GetVJDContPovNumber(device_id) } as u32;

                let hats = if hat_disc_count > 0 {
                    (1..=hat_disc_count)
                        .map(|hat_id| Hat {
                            id: hat_id as u8,
                            state: HatState::Discrete(FourWayHat::Centered),
                        })
                        .collect()
                } else if hat_cont_count > 0 {
                    (1..=hat_cont_count)
                        .map(|hat_id| Hat {
                            id: hat_id as u8,
                            state: HatState::Continuous(u32::MAX),
                        })
                        .collect()
                } else {
                    Vec::new()
                };

                trace!("Device {} hat switch count: {}", device_id, hats.len());

                self.devices.push(Device {
                    id: device_id,
                    buttons,
                    axes,
                    hats,
                })
            }
        }
    }

    #[profiling::function]
    fn acquire_device(&self, device_id: u32) -> Result<(), Error> {
        unsafe {
            let result = self.ffi.AcquireVJD(device_id);
            if result == 1 {
                trace!("Acquired device {}", device_id);
                Ok(())
            } else {
                Err(Error::Ffi(FFIError::DeviceCouldNotBeAcquired(
                    device_id, result as u32
                )))
            }
        }
    }

    #[profiling::function]
    fn relinquish_device(&self, device_id: u32) {
        unsafe {
            self.ffi.RelinquishVJD(device_id);
            println!("Relinquished device {}", device_id);
        }
    }

    #[profiling::function]
    fn update_device_data(
        ffi: &vjoy_sys::vJoyInterface,
        device_id: u32,
        data: &mut JOYSTICK_POSITION,
    ) -> Result<(), Error> {
        unsafe {
            let ptr = data as *mut JOYSTICK_POSITION;
            let result = ffi.UpdateVJD(device_id, ptr);
            if result != 1 {
                let device_state = Self::get_device_ffi_status(ffi, device_id);
                return Err(Error::Ffi(FFIError::DeviceDataCouldNotBeUpdated(
                    device_id,
                    device_state,
                )));
            }
        }
        Ok(())
    }

    #[profiling::function]
    fn set_button(
        ffi: &vjoy_sys::vJoyInterface,
        device_id: u32,
        button_id: u8,
        state: ButtonState,
    ) -> Result<(), Error> {
        unsafe {
            let result = ffi.SetBtn(state as i32, device_id, button_id);
            if result != 1 {
                let device_state = Self::get_device_ffi_status(ffi, device_id);
                return Err(Error::Ffi(FFIError::ButtonCouldNotBeSet(
                    device_id,
                    button_id,
                    device_state,
                )));
            }
        }
        Ok(())
    }

    #[profiling::function]
    fn set_axis(
        ffi: &vjoy_sys::vJoyInterface,
        device_id: u32,
        axis_id: u32,
        value: i32,
    ) -> Result<(), Error> {
        unsafe {
            let axis_index = (axis_id - 1) as usize;
            let axis_hid = AXES_HID_USAGE[axis_index];
            let result = ffi.SetAxis(value, device_id, axis_hid);
            if result != 1 {
                let device_state = Self::get_device_ffi_status(ffi, device_id);
                return Err(Error::Ffi(FFIError::AxisCouldNotBeSet(
                    device_id,
                    axis_id,
                    device_state,
                )));
            }
        }
        Ok(())
    }

    #[profiling::function]
    fn set_hat(
        ffi: &vjoy_sys::vJoyInterface,
        device_id: u32,
        hat_id: u8,
        state: HatState,
    ) -> Result<(), Error> {
        unsafe {
            let result = match state {
                HatState::Discrete(disc) => ffi.SetDiscPov(disc as i32, device_id, hat_id),
                HatState::Continuous(cont) => ffi.SetContPov(cont, device_id, hat_id),
            };
            if result != 1 {
                let device_state = Self::get_device_ffi_status(ffi, device_id);
                return Err(Error::Ffi(FFIError::HatCouldNotBeSet(
                    device_id,
                    hat_id,
                    device_state,
                )));
            }
        }
        Ok(())
    }

    #[profiling::function]
    fn get_device_ffi_status(ffi: &vjoy_sys::vJoyInterface, device_id: u32) -> VjdStat {
        unsafe { ffi.GetVJDStatus(device_id) }
    }
}

impl Drop for VJoy {
    #[profiling::function]
    fn drop(&mut self) {
        for device in &self.devices {
            self.relinquish_device(device.id);
        }
    }
}
