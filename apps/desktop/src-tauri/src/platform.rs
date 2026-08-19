use std::process::Command;

/// Prevents background CLI children from creating visible console windows in
/// Windows GUI builds. This must be applied to every command spawned directly
/// by Rust; the Node launcher separately uses `windowsHide` for its child.
pub fn configure_background_command(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    #[cfg(not(windows))]
    let _ = command;
}
