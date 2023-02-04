#![no_std]
use asr::{
    timer,
    timer::TimerState,
    watcher::Watcher,
    Address,
    Process,
    signature::Signature,
};

#[cfg(all(not(test), target_arch = "wasm32"))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    core::arch::wasm32::unreachable()
}

static AUTOSPLITTER: spinning_top::Spinlock<State> = spinning_top::const_spinlock(State {
    game: None,
    sigscans: None,
    watchers: Watchers {
        is_loading: Watcher::new(),
        fade: Watcher::new(),
    },
    // settings: None,
});

struct State {
    game: Option<ProcessInfo>,
    sigscans: Option<SigScan>,
    watchers: Watchers,
    // settings: Settings,
}

struct Watchers {
    is_loading: Watcher<u8>,
    fade: Watcher<f32>,
}

struct ProcessInfo {
    game: Process,
    main_module_base: Address,
    //main_module_size: u64,
}

impl State {
    fn attach_process() -> Option<ProcessInfo> {
        let game_names = ["Need For Speed The Run.exe"];
        let mut proc: Option<Process> = None;
        let mut curgamename: &str = "";
        
        for name in game_names {
            let iproc = Process::attach(name);
            if iproc.is_some() {
                proc = iproc;
                curgamename = name;
                break;
            }
        }
        
        let Some(game) = proc else { return None };

        // Sets the tick rate to 60hz because I think 120 (the default value) is overkill for this autosplitter
        asr::set_tick_rate(60.0);

        let main_module_base = game.get_module_address(curgamename).ok()?;
        //let main_module_size = game.get_module_size(curgamename).ok()?;

        Some(ProcessInfo {
            game,
            main_module_base,
            //main_module_size, // currently broken in livesplit classic
        })
    }

    fn update(&mut self) {
        // Checks is LiveSplit is currently attached to a target process and runs attach_process() otherwise
        if self.game.is_none() {
            self.game = State::attach_process()
        }

        let Some(game) = &self.game else {
            return;
        };
        let proc = &game.game;
        if !proc.is_open() {
            self.game = None;
            return;
        };

        // Update
        // Look for valid sigscans and performs sigscan if necessary
        let Some(addresses) = &self.sigscans else { self.sigscans = SigScan::new(proc, game.main_module_base); return; };

        // Update the watchers variables
        let Some(is_loading) = self.watchers.is_loading.update(proc.read(addresses.is_loading).ok()) else { return };
        let Some(fade) = self.watchers.fade.update(proc.read(addresses.fade).ok()) else { return };

        // Splitting logic
        match timer::state() {
            TimerState::Running => {
                if is_loading.current == 0 {
                    timer::resume_game_time()
                } else if fade.current == 1.0 {
                    timer::pause_game_time()
                } else {
                    timer::resume_game_time()
                }
            }
            _ => {}
        }
    }
}

#[no_mangle]
pub extern "C" fn update() {
    AUTOSPLITTER.lock().update();
}

struct SigScan {
    is_loading: Address,
    fade: Address,
}

impl SigScan {
    fn new(process: &Process, addr: Address) -> Option<Self> {
        let size = 0x3200000; // temporary hack until we can actually query ModuleMemorySize

        // Sigscan for easy value
        let sig: Signature<8> = Signature::new("39 05 ???????? 6A 00");
        let mut scan = sig.scan_process_range(process, addr, size)?;
        let is_loading = process.read::<u32>(Address(scan.0 + 2)).ok()?;

        // Sligthly harder to find value
        let sig: Signature<14> = Signature::new("E8 ???????? E8 ???????? 38 5C 24 14");
        scan = sig.scan_process_range(process, addr, size)?;
        let address = process.read::<u32>(Address(scan.0 + 1)).ok()?;
        let fade = process.read::<u32>(Address(scan.0 + 0x5 + address as u64 + 0x14)).ok()?;

        Some(Self {
            is_loading: Address(is_loading as u64),
            fade: Address(fade as u64 + 0x4),
        })
    }
}