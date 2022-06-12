### Description

<!-- Please include a brief description of what is being added/changed -->

e.g: This PR implements the `foobar` extension, based off the GDB documentation [here](https://sourceware.org/gdb/onlinedocs/gdb/Remote-Protocol.html).

Closes #(issue number) <!-- if appropriate -->

### API Stability

- [ ] This PR does not require a breaking API change

<!-- If it does require making a breaking API change, please elaborate why -->

### Checklist

<!-- CI takes care of a lot of things, but there are some things that have yet to be automated -->

- Documentation
  - [ ] Ensured any public-facing `rustdoc` formatting looks good (via `cargo doc`)
  - [ ] (if appropriate) Added feature to "Debugging Features" in README.md
- Validation
  - [ ] Included output of running `examples/armv4t` with `RUST_LOG=trace` + any relevant GDB output under the "Validation" section below
  - [ ] Included output of running `./example_no_std/check_size.sh` before/after changes under the "Validation" section below
- _If implementing a new protocol extension IDET_
  - [ ] Included a basic sample implementation in `examples/armv4t`
  - [ ] IDET can be optimized out (confirmed via `./example_no_std/check_size.sh`)
  - [ ] **OR** implementation requires introducing non-optional binary bloat (please elaborate under "Description")
- _If upstreaming an `Arch` implementation_
  - [ ] I have tested this code in my project, and to the best of my knowledge, it is working as intended.

<!-- Oh, and if you're integrating `gdbstub` in an open-source project, do consider updating the README.md's "Real World Examples" section to link back to your project! -->

### Validation

<!-- example output, from https://github.com/daniel5151/gdbstub/pull/54 -->

<details>
<summary>GDB output</summary>

```
!!!!! EXAMPLE OUTPUT !!!!!

(gdb) info mem
Using memory regions provided by the target.
Num Enb Low Addr   High Addr  Attrs
0   y  	0x00000000 0x100000000 rw nocache
```

</details>

<details>
<summary>armv4t output</summary>

```
!!!!! EXAMPLE OUTPUT !!!!!

    Finished dev [unoptimized + debuginfo] target(s) in 0.01s
     Running `target/debug/examples/armv4t`
loading section ".text" into memory from [0x55550000..0x55550078]
Setting PC to 0x55550000
Waiting for a GDB connection on "127.0.0.1:9001"...
Debugger connected from 127.0.0.1:37142
 TRACE gdbstub::gdbstub_impl > <-- +
 TRACE gdbstub::gdbstub_impl > <-- $qSupported:multiprocess+;swbreak+;hwbreak+;qRelocInsn+;fork-events+;vfork-events+;exec-events+;vContSupported+;QThreadEvents+;no-resumed+;xmlRegisters=i386#6a
 TRACE gdbstub::protocol::response_writer > --> $PacketSize=1000;vContSupported+;multiprocess+;QStartNoAckMode+;ReverseContinue+;ReverseStep+;QDisableRandomization+;QEnvironmentHexEncoded+;QEnvironmentUnset+;QEnvironmentReset+;QStartupWithShell+;QSetWorkingDir+;swbreak+;hwbreak+;qXfer:features:read+;qXfer:memory-map:read+#e4
 TRACE gdbstub::gdbstub_impl              > <-- +
 TRACE gdbstub::gdbstub_impl              > <-- $vMustReplyEmpty#3a
 INFO  gdbstub::gdbstub_impl              > Unknown command: vMustReplyEmpty
 TRACE gdbstub::protocol::response_writer > --> $#00
 TRACE gdbstub::gdbstub_impl              > <-- +
 TRACE gdbstub::gdbstub_impl              > <-- $QStartNoAckMode#b0
 TRACE gdbstub::protocol::response_writer > --> $OK#9a
 TRACE gdbstub::gdbstub_impl              > <-- +
 TRACE gdbstub::gdbstub_impl              > <-- $Hgp0.0#ad
 TRACE gdbstub::protocol::response_writer > --> $OK#9a
 TRACE gdbstub::gdbstub_impl              > <-- $qXfer:features:read:target.xml:0,ffb#79
 TRACE gdbstub::protocol::response_writer > --> $l<target version="1.0"><!-- custom override string --><architecture>armv4t</architecture></target>#bb
 TRACE gdbstub::gdbstub_impl              > <-- $qTStatus#49
 INFO  gdbstub::gdbstub_impl              > Unknown command: qTStatus
 TRACE gdbstub::protocol::response_writer > --> $#00
 TRACE gdbstub::gdbstub_impl              > <-- $?#3f
 TRACE gdbstub::protocol::response_writer > --> $S05#b8
 TRACE gdbstub::gdbstub_impl              > <-- $qfThreadInfo#bb
 TRACE gdbstub::protocol::response_writer > --> $mp01.01#cd
 TRACE gdbstub::gdbstub_impl              > <-- $qsThreadInfo#c8
 TRACE gdbstub::protocol::response_writer > --> $l#6c
 TRACE gdbstub::gdbstub_impl              > <-- $qAttached:1#fa
GDB queried if it was attached to a process with PID 1
 TRACE gdbstub::protocol::response_writer > --> $1#31
 TRACE gdbstub::gdbstub_impl              > <-- $Hc-1#09
 TRACE gdbstub::protocol::response_writer > --> $OK#9a
 TRACE gdbstub::gdbstub_impl              > <-- $qC#b4
 INFO  gdbstub::gdbstub_impl              > Unknown command: qC
 TRACE gdbstub::protocol::response_writer > --> $#00
 TRACE gdbstub::gdbstub_impl              > <-- $g#67
 TRACE gdbstub::protocol::response_writer > --> $00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000107856341200005555xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx10000000#66
 TRACE gdbstub::gdbstub_impl              > <-- $qfThreadInfo#bb
 TRACE gdbstub::protocol::response_writer > --> $mp01.01#cd
 TRACE gdbstub::gdbstub_impl              > <-- $qsThreadInfo#c8
 TRACE gdbstub::protocol::response_writer > --> $l#6c
 TRACE gdbstub::gdbstub_impl              > <-- $qXfer:memory-map:read::0,ffb#18
 TRACE gdbstub::protocol::response_writer > --> $l<?xml version="1.0"?>
<!DOCTYPE memory-map
    PUBLIC "+//IDN gnu.org//DTD GDB Memory Map V1.0//EN"
            "http://sourceware.org/gdb/gdb-memory-map.dtd">
<memory-map>
    <memory type="ram" start="0x0" length="0x100000000"/>
</memory-map>#75
 TRACE gdbstub::gdbstub_impl              > <-- $m55550000,4#61
 TRACE gdbstub::protocol::response_writer > --> $04b02de5#26
 TRACE gdbstub::gdbstub_impl              > <-- $m5554fffc,4#35
 TRACE gdbstub::protocol::response_writer > --> $00000000#7e
 TRACE gdbstub::gdbstub_impl              > <-- $m55550000,4#61
 TRACE gdbstub::protocol::response_writer > --> $04b02de5#26
 TRACE gdbstub::gdbstub_impl              > <-- $m5554fffc,4#35
 TRACE gdbstub::protocol::response_writer > --> $00000000#7e
 TRACE gdbstub::gdbstub_impl              > <-- $m55550000,2#5f
 TRACE gdbstub::protocol::response_writer > --> $04b0#f6
 TRACE gdbstub::gdbstub_impl              > <-- $m5554fffe,2#35
 TRACE gdbstub::protocol::response_writer > --> $0000#7a
 TRACE gdbstub::gdbstub_impl              > <-- $m5554fffc,2#33
 TRACE gdbstub::protocol::response_writer > --> $0000#7a
 TRACE gdbstub::gdbstub_impl              > <-- $m55550000,2#5f
 TRACE gdbstub::protocol::response_writer > --> $04b0#f6
 TRACE gdbstub::gdbstub_impl              > <-- $m5554fffe,2#35
 TRACE gdbstub::protocol::response_writer > --> $0000#7a
 TRACE gdbstub::gdbstub_impl              > <-- $m5554fffc,2#33
 TRACE gdbstub::protocol::response_writer > --> $0000#7a
 TRACE gdbstub::gdbstub_impl              > <-- $m55550000,4#61
 TRACE gdbstub::protocol::response_writer > --> $04b02de5#26
 TRACE gdbstub::gdbstub_impl              > <-- $m5554fffc,4#35
 TRACE gdbstub::protocol::response_writer > --> $00000000#7e
 TRACE gdbstub::gdbstub_impl              > <-- $m55550000,4#61
 TRACE gdbstub::protocol::response_writer > --> $04b02de5#26
 TRACE gdbstub::gdbstub_impl              > <-- $m5554fffc,4#35
 TRACE gdbstub::protocol::response_writer > --> $00000000#7e
 TRACE gdbstub::gdbstub_impl              > <-- $m55550000,4#61
 TRACE gdbstub::protocol::response_writer > --> $04b02de5#26
 TRACE gdbstub::gdbstub_impl              > <-- $m55550000,4#61
 TRACE gdbstub::protocol::response_writer > --> $04b02de5#26
 TRACE gdbstub::gdbstub_impl              > <-- $m55550000,4#61
 TRACE gdbstub::protocol::response_writer > --> $04b02de5#26
 TRACE gdbstub::gdbstub_impl              > <-- $m0,4#fd
 TRACE gdbstub::protocol::response_writer > --> $00000000#7e
```
</details>

<details>
<summary>Before/After `./example_no_std/check_size.sh` output</summary>

### Before

```
!!!!! EXAMPLE OUTPUT !!!!!

target/release/gdbstub-nostd  :
section               size    addr
.interp                 28     680
.note.gnu.build-id      36     708
.note.ABI-tag           32     744
.gnu.hash               36     776
.dynsym                360     816
.dynstr                193    1176
.gnu.version            30    1370
.gnu.version_r          48    1400
.rela.dyn              408    1448
.init                   27    4096
.plt                    16    4128
.plt.got                 8    4144
.text                15253    4160
.fini                   13   19416
.rodata                906   20480
.eh_frame_hdr          284   21388
.eh_frame             1432   21672
.init_array              8   28072
.fini_array              8   28080
.dynamic               448   28088
.got                   136   28536
.data                    8   28672
.bss                     8   28680
.comment                43       0
Total                19769
```

### After

```
!!!!! EXAMPLE OUTPUT !!!!!

target/release/gdbstub-nostd  :
section               size    addr
.interp                 28     680
.note.gnu.build-id      36     708
.note.ABI-tag           32     744
.gnu.hash               36     776
.dynsym                360     816
.dynstr                193    1176
.gnu.version            30    1370
.gnu.version_r          48    1400
.rela.dyn              408    1448
.init                   27    4096
.plt                    16    4128
.plt.got                 8    4144
.text                15253    4160
.fini                   13   19416
.rodata                906   20480
.eh_frame_hdr          284   21388
.eh_frame             1432   21672
.init_array              8   28072
.fini_array              8   28080
.dynamic               448   28088
.got                   136   28536
.data                    8   28672
.bss                     8   28680
.comment                43       0
Total                19769
```

</details>
