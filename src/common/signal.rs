/// Cross-platform signal numbers defined by the GDB Remote Serial Protocol.
///
/// Transcribed from <https://github.com/bminor/binutils-gdb/blob/master/include/gdb/signals.def>
#[repr(transparent)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Signal(pub u8);

#[allow(clippy::upper_case_acronyms)]
#[allow(non_camel_case_types)]
#[rustfmt::skip]
impl Signal {
    #[doc = "Signal 0 (shouldn't be used)"]    pub const SIGZERO:    Self = Self(0);
    #[doc = "Hangup"]                          pub const SIGHUP:     Self = Self(1);
    #[doc = "Interrupt"]                       pub const SIGINT:     Self = Self(2);
    #[doc = "Quit"]                            pub const SIGQUIT:    Self = Self(3);
    #[doc = "Illegal instruction"]             pub const SIGILL:     Self = Self(4);
    #[doc = "Trace/breakpoint trap"]           pub const SIGTRAP:    Self = Self(5);
    #[doc = "Aborted"]                         pub const SIGABRT:    Self = Self(6);
    #[doc = "Emulation trap"]                  pub const SIGEMT:     Self = Self(7);
    #[doc = "Arithmetic exception"]            pub const SIGFPE:     Self = Self(8);
    #[doc = "Killed"]                          pub const SIGKILL:    Self = Self(9);
    #[doc = "Bus error"]                       pub const SIGBUS:     Self = Self(10);
    #[doc = "Segmentation fault"]              pub const SIGSEGV:    Self = Self(11);
    #[doc = "Bad system call"]                 pub const SIGSYS:     Self = Self(12);
    #[doc = "Broken pipe"]                     pub const SIGPIPE:    Self = Self(13);
    #[doc = "Alarm clock"]                     pub const SIGALRM:    Self = Self(14);
    #[doc = "Terminated"]                      pub const SIGTERM:    Self = Self(15);
    #[doc = "Urgent I/O condition"]            pub const SIGURG:     Self = Self(16);
    #[doc = "Stopped (signal)"]                pub const SIGSTOP:    Self = Self(17);
    #[doc = "Stopped (user)"]                  pub const SIGTSTP:    Self = Self(18);
    #[doc = "Continued"]                       pub const SIGCONT:    Self = Self(19);
    #[doc = "Child status changed"]            pub const SIGCHLD:    Self = Self(20);
    #[doc = "Stopped (tty input)"]             pub const SIGTTIN:    Self = Self(21);
    #[doc = "Stopped (tty output)"]            pub const SIGTTOU:    Self = Self(22);
    #[doc = "I/O possible"]                    pub const SIGIO:      Self = Self(23);
    #[doc = "CPU time limit exceeded"]         pub const SIGXCPU:    Self = Self(24);
    #[doc = "File size limit exceeded"]        pub const SIGXFSZ:    Self = Self(25);
    #[doc = "Virtual timer expired"]           pub const SIGVTALRM:  Self = Self(26);
    #[doc = "Profiling timer expired"]         pub const SIGPROF:    Self = Self(27);
    #[doc = "Window size changed"]             pub const SIGWINCH:   Self = Self(28);
    #[doc = "Resource lost"]                   pub const SIGLOST:    Self = Self(29);
    #[doc = "User defined signal 1"]           pub const SIGUSR1:    Self = Self(30);
    #[doc = "User defined signal 2"]           pub const SIGUSR2:    Self = Self(31);
    #[doc = "Power fail/restart"]              pub const SIGPWR:     Self = Self(32);
    /* Similar to SIGIO.  Perhaps they should have the same number. */
    #[doc = "Pollable event occurred"]         pub const SIGPOLL:    Self = Self(33);
    #[doc = "SIGWIND"]                         pub const SIGWIND:    Self = Self(34);
    #[doc = "SIGPHONE"]                        pub const SIGPHONE:   Self = Self(35);
    #[doc = "Process's LWPs are blocked"]      pub const SIGWAITING: Self = Self(36);
    #[doc = "Signal LWP"]                      pub const SIGLWP:     Self = Self(37);
    #[doc = "Swap space dangerously low"]      pub const SIGDANGER:  Self = Self(38);
    #[doc = "Monitor mode granted"]            pub const SIGGRANT:   Self = Self(39);
    #[doc = "Need to relinquish monitor mode"] pub const SIGRETRACT: Self = Self(40);
    #[doc = "Monitor mode data available"]     pub const SIGMSG:     Self = Self(41);
    #[doc = "Sound completed"]                 pub const SIGSOUND:   Self = Self(42);
    #[doc = "Secure attention"]                pub const SIGSAK:     Self = Self(43);
    #[doc = "SIGPRIO"]                         pub const SIGPRIO:    Self = Self(44);
    #[doc = "Real-time event 33"]              pub const SIG33:      Self = Self(45);
    #[doc = "Real-time event 34"]              pub const SIG34:      Self = Self(46);
    #[doc = "Real-time event 35"]              pub const SIG35:      Self = Self(47);
    #[doc = "Real-time event 36"]              pub const SIG36:      Self = Self(48);
    #[doc = "Real-time event 37"]              pub const SIG37:      Self = Self(49);
    #[doc = "Real-time event 38"]              pub const SIG38:      Self = Self(50);
    #[doc = "Real-time event 39"]              pub const SIG39:      Self = Self(51);
    #[doc = "Real-time event 40"]              pub const SIG40:      Self = Self(52);
    #[doc = "Real-time event 41"]              pub const SIG41:      Self = Self(53);
    #[doc = "Real-time event 42"]              pub const SIG42:      Self = Self(54);
    #[doc = "Real-time event 43"]              pub const SIG43:      Self = Self(55);
    #[doc = "Real-time event 44"]              pub const SIG44:      Self = Self(56);
    #[doc = "Real-time event 45"]              pub const SIG45:      Self = Self(57);
    #[doc = "Real-time event 46"]              pub const SIG46:      Self = Self(58);
    #[doc = "Real-time event 47"]              pub const SIG47:      Self = Self(59);
    #[doc = "Real-time event 48"]              pub const SIG48:      Self = Self(60);
    #[doc = "Real-time event 49"]              pub const SIG49:      Self = Self(61);
    #[doc = "Real-time event 50"]              pub const SIG50:      Self = Self(62);
    #[doc = "Real-time event 51"]              pub const SIG51:      Self = Self(63);
    #[doc = "Real-time event 52"]              pub const SIG52:      Self = Self(64);
    #[doc = "Real-time event 53"]              pub const SIG53:      Self = Self(65);
    #[doc = "Real-time event 54"]              pub const SIG54:      Self = Self(66);
    #[doc = "Real-time event 55"]              pub const SIG55:      Self = Self(67);
    #[doc = "Real-time event 56"]              pub const SIG56:      Self = Self(68);
    #[doc = "Real-time event 57"]              pub const SIG57:      Self = Self(69);
    #[doc = "Real-time event 58"]              pub const SIG58:      Self = Self(70);
    #[doc = "Real-time event 59"]              pub const SIG59:      Self = Self(71);
    #[doc = "Real-time event 60"]              pub const SIG60:      Self = Self(72);
    #[doc = "Real-time event 61"]              pub const SIG61:      Self = Self(73);
    #[doc = "Real-time event 62"]              pub const SIG62:      Self = Self(74);
    #[doc = "Real-time event 63"]              pub const SIG63:      Self = Self(75);
    /* Used internally by Solaris threads.  See signal(5) on Solaris. */
    #[doc = "LWP internal signal"]             pub const SIGCANCEL:  Self = Self(76);
    /* Yes, this pains me, too.  But LynxOS didn't have SIG32, and now
    GNU/Linux does, and we can't disturb the numbering, since it's
    part of the remote protocol.  Note that in some GDB's
    GDB_SIGNAL_REALTIME_32 is number 76.  */
    #[doc = "Real-time event 32"]              pub const SIG32:      Self = Self(77);
    /* Yet another pain, IRIX 6 has SIG64. */
    #[doc = "Real-time event 64"]              pub const SIG64:      Self = Self(78);
    /* Yet another pain, GNU/Linux MIPS might go up to 128. */
    #[doc = "Real-time event 65"]              pub const SIG65:      Self = Self(79);
    #[doc = "Real-time event 66"]              pub const SIG66:      Self = Self(80);
    #[doc = "Real-time event 67"]              pub const SIG67:      Self = Self(81);
    #[doc = "Real-time event 68"]              pub const SIG68:      Self = Self(82);
    #[doc = "Real-time event 69"]              pub const SIG69:      Self = Self(83);
    #[doc = "Real-time event 70"]              pub const SIG70:      Self = Self(84);
    #[doc = "Real-time event 71"]              pub const SIG71:      Self = Self(85);
    #[doc = "Real-time event 72"]              pub const SIG72:      Self = Self(86);
    #[doc = "Real-time event 73"]              pub const SIG73:      Self = Self(87);
    #[doc = "Real-time event 74"]              pub const SIG74:      Self = Self(88);
    #[doc = "Real-time event 75"]              pub const SIG75:      Self = Self(89);
    #[doc = "Real-time event 76"]              pub const SIG76:      Self = Self(90);
    #[doc = "Real-time event 77"]              pub const SIG77:      Self = Self(91);
    #[doc = "Real-time event 78"]              pub const SIG78:      Self = Self(92);
    #[doc = "Real-time event 79"]              pub const SIG79:      Self = Self(93);
    #[doc = "Real-time event 80"]              pub const SIG80:      Self = Self(94);
    #[doc = "Real-time event 81"]              pub const SIG81:      Self = Self(95);
    #[doc = "Real-time event 82"]              pub const SIG82:      Self = Self(96);
    #[doc = "Real-time event 83"]              pub const SIG83:      Self = Self(97);
    #[doc = "Real-time event 84"]              pub const SIG84:      Self = Self(98);
    #[doc = "Real-time event 85"]              pub const SIG85:      Self = Self(99);
    #[doc = "Real-time event 86"]              pub const SIG86:      Self = Self(100);
    #[doc = "Real-time event 87"]              pub const SIG87:      Self = Self(101);
    #[doc = "Real-time event 88"]              pub const SIG88:      Self = Self(102);
    #[doc = "Real-time event 89"]              pub const SIG89:      Self = Self(103);
    #[doc = "Real-time event 90"]              pub const SIG90:      Self = Self(104);
    #[doc = "Real-time event 91"]              pub const SIG91:      Self = Self(105);
    #[doc = "Real-time event 92"]              pub const SIG92:      Self = Self(106);
    #[doc = "Real-time event 93"]              pub const SIG93:      Self = Self(107);
    #[doc = "Real-time event 94"]              pub const SIG94:      Self = Self(108);
    #[doc = "Real-time event 95"]              pub const SIG95:      Self = Self(109);
    #[doc = "Real-time event 96"]              pub const SIG96:      Self = Self(110);
    #[doc = "Real-time event 97"]              pub const SIG97:      Self = Self(111);
    #[doc = "Real-time event 98"]              pub const SIG98:      Self = Self(112);
    #[doc = "Real-time event 99"]              pub const SIG99:      Self = Self(113);
    #[doc = "Real-time event 100"]             pub const SIG100:     Self = Self(114);
    #[doc = "Real-time event 101"]             pub const SIG101:     Self = Self(115);
    #[doc = "Real-time event 102"]             pub const SIG102:     Self = Self(116);
    #[doc = "Real-time event 103"]             pub const SIG103:     Self = Self(117);
    #[doc = "Real-time event 104"]             pub const SIG104:     Self = Self(118);
    #[doc = "Real-time event 105"]             pub const SIG105:     Self = Self(119);
    #[doc = "Real-time event 106"]             pub const SIG106:     Self = Self(120);
    #[doc = "Real-time event 107"]             pub const SIG107:     Self = Self(121);
    #[doc = "Real-time event 108"]             pub const SIG108:     Self = Self(122);
    #[doc = "Real-time event 109"]             pub const SIG109:     Self = Self(123);
    #[doc = "Real-time event 110"]             pub const SIG110:     Self = Self(124);
    #[doc = "Real-time event 111"]             pub const SIG111:     Self = Self(125);
    #[doc = "Real-time event 112"]             pub const SIG112:     Self = Self(126);
    #[doc = "Real-time event 113"]             pub const SIG113:     Self = Self(127);
    #[doc = "Real-time event 114"]             pub const SIG114:     Self = Self(128);
    #[doc = "Real-time event 115"]             pub const SIG115:     Self = Self(129);
    #[doc = "Real-time event 116"]             pub const SIG116:     Self = Self(130);
    #[doc = "Real-time event 117"]             pub const SIG117:     Self = Self(131);
    #[doc = "Real-time event 118"]             pub const SIG118:     Self = Self(132);
    #[doc = "Real-time event 119"]             pub const SIG119:     Self = Self(133);
    #[doc = "Real-time event 120"]             pub const SIG120:     Self = Self(134);
    #[doc = "Real-time event 121"]             pub const SIG121:     Self = Self(135);
    #[doc = "Real-time event 122"]             pub const SIG122:     Self = Self(136);
    #[doc = "Real-time event 123"]             pub const SIG123:     Self = Self(137);
    #[doc = "Real-time event 124"]             pub const SIG124:     Self = Self(138);
    #[doc = "Real-time event 125"]             pub const SIG125:     Self = Self(139);
    #[doc = "Real-time event 126"]             pub const SIG126:     Self = Self(140);
    #[doc = "Real-time event 127"]             pub const SIG127:     Self = Self(141);

    #[doc = "Information request"]             pub const SIGINFO:    Self = Self(142);

    /* Some signal we don't know about. */
    #[doc = "Unknown signal"]                  pub const UNKNOWN:    Self = Self(143);

    /* Use whatever signal we use when one is not specifically specified
    (for passing to proceed and so on).  */
    #[doc = "Internal error: printing GDB_SIGNAL_DEFAULT"]
    pub const INTERNAL_DEFAULT: Self = Self(144);

    /* Mach exceptions.  In versions of GDB before 5.2, these were just before
    GDB_SIGNAL_INFO if you were compiling on a Mach host (and missing
    otherwise).  */
    #[doc = "Could not access memory"]         pub const EXC_BAD_ACCESS:      Self = Self(145);
    #[doc = "Illegal instruction/operand"]     pub const EXC_BAD_INSTRUCTION: Self = Self(146);
    #[doc = "Arithmetic exception"]            pub const EXC_ARITHMETIC:      Self = Self(147);
    #[doc = "Emulation instruction"]           pub const EXC_EMULATION:       Self = Self(148);
    #[doc = "Software generated exception"]    pub const EXC_SOFTWARE:        Self = Self(149);
    #[doc = "Breakpoint"]                      pub const EXC_BREAKPOINT:      Self = Self(150);

    #[doc = "librt internal signal"]           pub const SIGLIBRT:            Self = Self(151);
}

impl core::fmt::Display for Signal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[rustfmt::skip]
        let s = match *self {
            Signal::SIGZERO             => "SIGZERO - Signal 0",
            Signal::SIGHUP              => "SIGHUP - Hangup",
            Signal::SIGINT              => "SIGINT - Interrupt",
            Signal::SIGQUIT             => "SIGQUIT - Quit",
            Signal::SIGILL              => "SIGILL - Illegal instruction",
            Signal::SIGTRAP             => "SIGTRAP - Trace/breakpoint trap",
            Signal::SIGABRT             => "SIGABRT - Aborted",
            Signal::SIGEMT              => "SIGEMT - Emulation trap",
            Signal::SIGFPE              => "SIGFPE - Arithmetic exception",
            Signal::SIGKILL             => "SIGKILL - Killed",
            Signal::SIGBUS              => "SIGBUS - Bus error",
            Signal::SIGSEGV             => "SIGSEGV - Segmentation fault",
            Signal::SIGSYS              => "SIGSYS - Bad system call",
            Signal::SIGPIPE             => "SIGPIPE - Broken pipe",
            Signal::SIGALRM             => "SIGALRM - Alarm clock",
            Signal::SIGTERM             => "SIGTERM - Terminated",
            Signal::SIGURG              => "SIGURG - Urgent I/O condition",
            Signal::SIGSTOP             => "SIGSTOP - Stopped (signal)",
            Signal::SIGTSTP             => "SIGTSTP - Stopped (user)",
            Signal::SIGCONT             => "SIGCONT - Continued",
            Signal::SIGCHLD             => "SIGCHLD - Child status changed",
            Signal::SIGTTIN             => "SIGTTIN - Stopped (tty input)",
            Signal::SIGTTOU             => "SIGTTOU - Stopped (tty output)",
            Signal::SIGIO               => "SIGIO - I/O possible",
            Signal::SIGXCPU             => "SIGXCPU - CPU time limit exceeded",
            Signal::SIGXFSZ             => "SIGXFSZ - File size limit exceeded",
            Signal::SIGVTALRM           => "SIGVTALRM - Virtual timer expired",
            Signal::SIGPROF             => "SIGPROF - Profiling timer expired",
            Signal::SIGWINCH            => "SIGWINCH - Window size changed",
            Signal::SIGLOST             => "SIGLOST - Resource lost",
            Signal::SIGUSR1             => "SIGUSR1 - User defined signal 1",
            Signal::SIGUSR2             => "SIGUSR2 - User defined signal 2",
            Signal::SIGPWR              => "SIGPWR - Power fail/restart",
            Signal::SIGPOLL             => "SIGPOLL - Pollable event occurred",
            Signal::SIGWIND             => "SIGWIND - SIGWIND",
            Signal::SIGPHONE            => "SIGPHONE - SIGPHONE",
            Signal::SIGWAITING          => "SIGWAITING - Process's LWPs are blocked",
            Signal::SIGLWP              => "SIGLWP - Signal LWP",
            Signal::SIGDANGER           => "SIGDANGER - Swap space dangerously low",
            Signal::SIGGRANT            => "SIGGRANT - Monitor mode granted",
            Signal::SIGRETRACT          => "SIGRETRACT - Need to relinquish monitor mode",
            Signal::SIGMSG              => "SIGMSG - Monitor mode data available",
            Signal::SIGSOUND            => "SIGSOUND - Sound completed",
            Signal::SIGSAK              => "SIGSAK - Secure attention",
            Signal::SIGPRIO             => "SIGPRIO - SIGPRIO",
            Signal::SIG33               => "SIG33 - Real-time event 33",
            Signal::SIG34               => "SIG34 - Real-time event 34",
            Signal::SIG35               => "SIG35 - Real-time event 35",
            Signal::SIG36               => "SIG36 - Real-time event 36",
            Signal::SIG37               => "SIG37 - Real-time event 37",
            Signal::SIG38               => "SIG38 - Real-time event 38",
            Signal::SIG39               => "SIG39 - Real-time event 39",
            Signal::SIG40               => "SIG40 - Real-time event 40",
            Signal::SIG41               => "SIG41 - Real-time event 41",
            Signal::SIG42               => "SIG42 - Real-time event 42",
            Signal::SIG43               => "SIG43 - Real-time event 43",
            Signal::SIG44               => "SIG44 - Real-time event 44",
            Signal::SIG45               => "SIG45 - Real-time event 45",
            Signal::SIG46               => "SIG46 - Real-time event 46",
            Signal::SIG47               => "SIG47 - Real-time event 47",
            Signal::SIG48               => "SIG48 - Real-time event 48",
            Signal::SIG49               => "SIG49 - Real-time event 49",
            Signal::SIG50               => "SIG50 - Real-time event 50",
            Signal::SIG51               => "SIG51 - Real-time event 51",
            Signal::SIG52               => "SIG52 - Real-time event 52",
            Signal::SIG53               => "SIG53 - Real-time event 53",
            Signal::SIG54               => "SIG54 - Real-time event 54",
            Signal::SIG55               => "SIG55 - Real-time event 55",
            Signal::SIG56               => "SIG56 - Real-time event 56",
            Signal::SIG57               => "SIG57 - Real-time event 57",
            Signal::SIG58               => "SIG58 - Real-time event 58",
            Signal::SIG59               => "SIG59 - Real-time event 59",
            Signal::SIG60               => "SIG60 - Real-time event 60",
            Signal::SIG61               => "SIG61 - Real-time event 61",
            Signal::SIG62               => "SIG62 - Real-time event 62",
            Signal::SIG63               => "SIG63 - Real-time event 63",
            Signal::SIGCANCEL           => "SIGCANCEL - LWP internal signal",
            Signal::SIG32               => "SIG32 - Real-time event 32",
            Signal::SIG64               => "SIG64 - Real-time event 64",
            Signal::SIG65               => "SIG65 - Real-time event 65",
            Signal::SIG66               => "SIG66 - Real-time event 66",
            Signal::SIG67               => "SIG67 - Real-time event 67",
            Signal::SIG68               => "SIG68 - Real-time event 68",
            Signal::SIG69               => "SIG69 - Real-time event 69",
            Signal::SIG70               => "SIG70 - Real-time event 70",
            Signal::SIG71               => "SIG71 - Real-time event 71",
            Signal::SIG72               => "SIG72 - Real-time event 72",
            Signal::SIG73               => "SIG73 - Real-time event 73",
            Signal::SIG74               => "SIG74 - Real-time event 74",
            Signal::SIG75               => "SIG75 - Real-time event 75",
            Signal::SIG76               => "SIG76 - Real-time event 76",
            Signal::SIG77               => "SIG77 - Real-time event 77",
            Signal::SIG78               => "SIG78 - Real-time event 78",
            Signal::SIG79               => "SIG79 - Real-time event 79",
            Signal::SIG80               => "SIG80 - Real-time event 80",
            Signal::SIG81               => "SIG81 - Real-time event 81",
            Signal::SIG82               => "SIG82 - Real-time event 82",
            Signal::SIG83               => "SIG83 - Real-time event 83",
            Signal::SIG84               => "SIG84 - Real-time event 84",
            Signal::SIG85               => "SIG85 - Real-time event 85",
            Signal::SIG86               => "SIG86 - Real-time event 86",
            Signal::SIG87               => "SIG87 - Real-time event 87",
            Signal::SIG88               => "SIG88 - Real-time event 88",
            Signal::SIG89               => "SIG89 - Real-time event 89",
            Signal::SIG90               => "SIG90 - Real-time event 90",
            Signal::SIG91               => "SIG91 - Real-time event 91",
            Signal::SIG92               => "SIG92 - Real-time event 92",
            Signal::SIG93               => "SIG93 - Real-time event 93",
            Signal::SIG94               => "SIG94 - Real-time event 94",
            Signal::SIG95               => "SIG95 - Real-time event 95",
            Signal::SIG96               => "SIG96 - Real-time event 96",
            Signal::SIG97               => "SIG97 - Real-time event 97",
            Signal::SIG98               => "SIG98 - Real-time event 98",
            Signal::SIG99               => "SIG99 - Real-time event 99",
            Signal::SIG100              => "SIG100 - Real-time event 100",
            Signal::SIG101              => "SIG101 - Real-time event 101",
            Signal::SIG102              => "SIG102 - Real-time event 102",
            Signal::SIG103              => "SIG103 - Real-time event 103",
            Signal::SIG104              => "SIG104 - Real-time event 104",
            Signal::SIG105              => "SIG105 - Real-time event 105",
            Signal::SIG106              => "SIG106 - Real-time event 106",
            Signal::SIG107              => "SIG107 - Real-time event 107",
            Signal::SIG108              => "SIG108 - Real-time event 108",
            Signal::SIG109              => "SIG109 - Real-time event 109",
            Signal::SIG110              => "SIG110 - Real-time event 110",
            Signal::SIG111              => "SIG111 - Real-time event 111",
            Signal::SIG112              => "SIG112 - Real-time event 112",
            Signal::SIG113              => "SIG113 - Real-time event 113",
            Signal::SIG114              => "SIG114 - Real-time event 114",
            Signal::SIG115              => "SIG115 - Real-time event 115",
            Signal::SIG116              => "SIG116 - Real-time event 116",
            Signal::SIG117              => "SIG117 - Real-time event 117",
            Signal::SIG118              => "SIG118 - Real-time event 118",
            Signal::SIG119              => "SIG119 - Real-time event 119",
            Signal::SIG120              => "SIG120 - Real-time event 120",
            Signal::SIG121              => "SIG121 - Real-time event 121",
            Signal::SIG122              => "SIG122 - Real-time event 122",
            Signal::SIG123              => "SIG123 - Real-time event 123",
            Signal::SIG124              => "SIG124 - Real-time event 124",
            Signal::SIG125              => "SIG125 - Real-time event 125",
            Signal::SIG126              => "SIG126 - Real-time event 126",
            Signal::SIG127              => "SIG127 - Real-time event 127",
            Signal::SIGINFO             => "SIGINFO - Information request",
            Signal::UNKNOWN             => "UNKNOWN - Unknown signal",
            Signal::INTERNAL_DEFAULT    => "INTERNAL_DEFAULT - Internal error: printing GDB_SIGNAL_DEFAULT",
            Signal::EXC_BAD_ACCESS      => "EXC_BAD_ACCESS - Could not access memory",
            Signal::EXC_BAD_INSTRUCTION => "EXC_BAD_INSTRUCTION - Illegal instruction/operand",
            Signal::EXC_ARITHMETIC      => "EXC_ARITHMETIC - Arithmetic exception",
            Signal::EXC_EMULATION       => "EXC_EMULATION - Emulation instruction",
            Signal::EXC_SOFTWARE        => "EXC_SOFTWARE - Software generated exception",
            Signal::EXC_BREAKPOINT      => "EXC_BREAKPOINT - Breakpoint",
            Signal::SIGLIBRT            => "SIGLIBRT - librt internal signal",

            _ => "custom signal (not defined in GDB's signals.def file)"
        };

        write!(f, "{}", s)
    }
}
