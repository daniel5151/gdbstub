### Description

<!-- Please include a brief description of what is being added/changed -->

e.g: This PR implements the `foobar` extension, based off the GDB documentation [here](https://sourceware.org/gdb/onlinedocs/gdb/Remote-Protocol.html).

Closes #(issue number) <!-- if appropriate -->

### API Stability

- [ ] This PR does not require a breaking API change

<!-- If it does require making a breaking API change, please elaborate why -->

### Checklist

- Implementation
  - [ ] `cargo build` compiles without `errors` or `warnings`
  - [ ] `cargo clippy` runs without `errors` or `warnings`
  - [ ] `cargo fmt` was run
  - [ ] All tests pass
- Documentation
  - [ ] rustdoc + approprate inline code comments
  - [ ] Updated CHANGELOG.md
  - [ ] (if appropriate) Added feature to "Debugging Features" in README.md
- _If implementing a new protocol extension IDET_
  - [ ] Included a basic sample implementation in `examples/armv4t`
  - [ ] Included output of running `examples/armv4t` with `RUST_LOG=trace` + any relevant GDB output under the "Validation" section below
  - [ ] Confirmed that IDET can be optimized away (using `./scripts/test_dead_code_elim.sh` and/or `./example_no_std/check_size.sh`)
  - [ ] **OR** Implementation requires adding non-optional binary bloat (please elaborate under "Description")
- _If upstreaming an `Arch` implementation_
  - [ ] I have tested this code in my project, and to the best of my knowledge, it is working as intended.

<!-- If you are implementing `gdbstub` in an open-source project, consider updating the README.md's "Real World Examples" section to link back to your project! -->

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
