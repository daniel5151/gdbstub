use crate::emu::Emu;
use gdbstub::target;
use gdbstub::target::TargetResult;
use gdbstub::target::TargetError;
use gdbstub::target::ext::tracepoints::{Tracepoint, TracepointItem, NewTracepoint, DefineTracepoint, ExperimentStatus, FrameRequest, FrameDescription, TraceBuffer};
use managed::ManagedSlice;

use armv4t_emu::Cpu;
pub struct TraceFrame {
    number: Tracepoint,
    addr: u32,
    snapshot: Cpu,
}

impl target::ext::tracepoints::Tracepoints for Emu {
    fn tracepoints_init(&mut self) -> TargetResult<(), Self> {
        self.tracepoints.clear();
        self.traceframes.clear();
        Ok(())
    }

    fn tracepoint_create(&mut self, tp: NewTracepoint<u32>) -> TargetResult<(), Self> {
        self.tracepoints.insert(tp.addr, vec![TracepointItem::New(tp)]);
        Ok(())
    }

    fn tracepoint_define(&mut self, tp: DefineTracepoint<'_, u32>) -> TargetResult<(), Self> {
        self.tracepoints.get_mut(&tp.addr).map(move |existing| {
            existing.push(TracepointItem::Define(tp.get_owned()));
            ()
        }).ok_or_else(move || TargetError::Fatal("define on non-existing tracepoint"))
    }

    fn tracepoint_status(&self, tp: Tracepoint, addr: u32) -> TargetResult<(u64, u64), Self> {
        // We don't collect "real" trace buffer frames, so just report hit count
        // and say the number of bytes is always 0
        Ok((self.traceframes.iter().filter(|frame| frame.number.0 == tp.0).count() as u64, 0))
    }

    fn tracepoint_enumerate_start(&mut self) -> TargetResult<Option<TracepointItem<'_, u32>>, Self> {
        let tracepoints: Vec<_> = self.tracepoints.iter().flat_map(|(key, value)| {
            value.iter().map(|item| item.get_owned())
        }).collect();
        self.tracepoint_enumerate_machine = (tracepoints, 0);

        self.tracepoint_enumerate_step()
    }

    fn tracepoint_enumerate_step(
        &mut self,
    ) -> TargetResult<Option<TracepointItem<'_, u32>>, Self> {
        let (tracepoints, index) = &mut self.tracepoint_enumerate_machine;
        if let Some(item) = tracepoints.iter().nth(*index) {
            *index += 1;
            let item2: TracepointItem<'static, u32> = item.get_owned();
            dbg!(&item2);
            Ok(Some(item2))
        } else {
            Ok(None)
        }
    }

    fn trace_buffer_configure(&mut self, tb: TraceBuffer) -> TargetResult<(), Self> {
        // we don't collect a "real" trace buffer, so just ignore configuration attempts.
        Ok(())
    }

    fn trace_buffer_request(
        &mut self,
        offset: u64,
        len: usize,
        buf: &mut [u8],
    ) -> TargetResult<Option<usize>, Self> {
        // We don't have a "real" trace buffer, so fail all raw read requests.
        Ok(None)
    }

    fn trace_experiment_status(&self) -> TargetResult<ExperimentStatus<'_>, Self> {
        // For a bare-bones example, we don't provide in-depth status explanations.
        Ok(ExperimentStatus {
            running: self.tracing,
            explanations: ManagedSlice::Owned(vec![]),
        })
    }

   fn select_frame(
        &mut self,
        frame: FrameRequest<u32>,
        report: &mut dyn FnMut(FrameDescription),
   ) -> TargetResult<(), Self> {
       // For a bare-bones example, we only support `tfind <number>` style frame
       // selection and not the more complicated ones.
       match frame {
           FrameRequest::Select(n) => {
                self.selected_frame = self.traceframes.iter().nth(n as usize).map(|frame| {
                    (report)(FrameDescription::FrameNumber(Some(n)));
                    (report)(FrameDescription::Hit(frame.number));
                    Some(n as usize)
                }).unwrap_or_else(|| {
                    (report)(FrameDescription::FrameNumber(None));
                    None
                });
                Ok(())
           },
           _ => Err(TargetError::NonFatal),
       }
   }

   fn trace_experiment_start(&mut self) -> TargetResult<(), Self> {
       self.tracing = true;
       Ok(())
   }

   fn trace_experiment_stop(&mut self) -> TargetResult<(), Self> {
       self.tracing = false;
       Ok(())
   }
}

