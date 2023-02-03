use spinning_top::{Spinlock, const_spinlock};
use asr::{
    timer,
    //timer::TimerState,
    watcher::Watcher,
    Address, Process,
};

static AUTOSPLITTER: Spinlock<State> = const_spinlock(
    State {
        game: None,
        watchers: Watchers {
            is_loading: Watcher::new(),
            fade: Watcher::new(),
        },
        // settings: None,
    }
);    

struct State {
    game: Option<ProcessInfo>,
    watchers: Watchers,
    // settings: Settings,
}

struct Watchers {
    is_loading: Watcher<u8>,
    fade: Watcher<f32>,
    // other
}

struct ProcessInfo {
    game: Option<Process>,
    main_module_base: Address,
    //main_module_size: u64,
}

impl State {
    fn attach_process() -> Option<ProcessInfo> {
        let game_name = "Need For Speed The Run.exe";
        let game_process = Process::attach(&game_name);

        if game_process.is_none() {
            return None;
        }

        let Some(proc) = game_process else {
            return None;
        };

        let game_main_module_base = proc.get_module_address(&game_name).ok().unwrap();
        //let game_main_module_size = proc.get_module_size(&game_name).ok().unwrap();

        Some(ProcessInfo {
            game: Some(proc),
            main_module_base: game_main_module_base,
            //main_module_size: game_main_module_size,
        })
    }

    fn check_if_attached(&self) -> bool {
        if self.game.is_none() {
            return false;
        }

        let Some(game) = &self.game else { return false };
        let Some(proc) = &game.game else {return false};
        proc.is_open()
    }

    fn update(&mut self) {
        // Attaching to target process
        if !self.check_if_attached() { self.game = State::attach_process() }
        if !self.check_if_attached() { return; }

        // Updating the watchers
        let game = self.game.as_ref().unwrap().game.as_ref().unwrap();
        let Address(base_address) = self.game.as_ref().unwrap().main_module_base;
        let is_loading = self.watchers.is_loading.update(game.read(Address(base_address + 0x24682E4)).ok()).unwrap();
        let fade = self.watchers.fade.update(game.read(Address(base_address + 0x248BBC4)).ok()).unwrap();

        if is_loading.current == 0 {
            timer::resume_game_time()
        } else if fade.current == 1.0 {
            timer::pause_game_time()
        } else {
            timer::resume_game_time()
        }
    }
}

#[no_mangle]
pub extern "C" fn update() {
    AUTOSPLITTER.lock().update();
}