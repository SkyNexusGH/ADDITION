//! Trainers are for single-player games. Touching a game protected by
//! kernel anti-cheat gets accounts banned, so the engine refuses to attach when
//! it sees one.

use crate::memory::{Module, ProcessInfo};

/// Services/launchers that run alongside protected games.
const PROCESSES: &[(&str, &str)] = &[
    ("easyanticheat.exe", "Easy Anti-Cheat"),
    ("easyanticheat_eos.exe", "Easy Anti-Cheat"),
    ("beservice.exe", "BattlEye"),
    ("beservice_x64.exe", "BattlEye"),
    ("vgc.exe", "Riot Vanguard"),
    ("vgtray.exe", "Riot Vanguard"),
    ("faceit.exe", "FACEIT"),
    ("faceitservice.exe", "FACEIT"),
    ("eaanticheat.gameservice.exe", "EA Javelin"),
    ("gameguard.des", "nProtect GameGuard"),
    ("xigncode3.exe", "XIGNCODE3"),
];

/// DLLs that protected games load into themselves.
const MODULES: &[(&str, &str)] = &[
    ("easyanticheat.dll", "Easy Anti-Cheat"),
    ("easyanticheat_x64.dll", "Easy Anti-Cheat"),
    ("easyanticheat_x86.dll", "Easy Anti-Cheat"),
    ("beclient.dll", "BattlEye"),
    ("beclient_x64.dll", "BattlEye"),
    ("eac_launcher.dll", "Easy Anti-Cheat"),
    ("eaanticheat.gameservice.dll", "EA Javelin"),
    ("randgrid.sys", "Ricochet"),
];

pub fn detect_in_processes(procs: &[ProcessInfo]) -> Option<&'static str> {
    procs.iter().find_map(|p| lookup(PROCESSES, &p.name))
}

pub fn detect_in_modules(mods: &[Module]) -> Option<&'static str> {
    mods.iter().find_map(|m| lookup(MODULES, &m.name))
}

fn lookup(table: &[(&str, &'static str)], name: &str) -> Option<&'static str> {
    table.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map(|(_, label)| *label)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let procs = vec![ProcessInfo { pid: 1, name: "EasyAntiCheat.exe".into() }];
        assert_eq!(detect_in_processes(&procs), Some("Easy Anti-Cheat"));
        let mods = vec![Module { name: "BEClient_x64.dll".into(), base: 0, size: 1 }];
        assert_eq!(detect_in_modules(&mods), Some("BattlEye"));
        assert_eq!(detect_in_modules(&[]), None);
    }
}
