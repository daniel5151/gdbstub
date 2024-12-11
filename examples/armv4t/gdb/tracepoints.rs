use crate::emu::Emu;
use gdbstub::target;
use gdbstub::target::TargetResult;
use gdbstub::target::TargetError;
use gdbstub::target::ext::tracepoints::{Tracepoint, TracepointItem, NewTracepoint, DefineTracepoint, ExperimentStatus, FrameRequest, FrameDescription, TraceBuffer, ExperimentExplanation, TracepointAction};
use managed::ManagedSlice;

use armv4t_emu::Cpu;
#[derive(Debug)]
pub struct TraceFrame {
    pub number: Tracepoint,
    pub addr: u32,
    pub snapshot: Cpu,
}

impl target::ext::tracepoints::Tracepoints for Emu {
    fn tracepoints_init(&mut self) -> TargetResult<(), Self> {
        self.tracepoints.clear();
        self.traceframes.clear();
        Ok(())
    }

    fn tracepoint_create(&mut self, tp: NewTracepoint<u32>) -> TargetResult<(), Self> {
        self.tracepoints.insert(tp.number, vec![TracepointItem::New(tp)]);
        Ok(())
    }

    fn tracepoint_define(&mut self, tp: DefineTracepoint<'_, u32>) -> TargetResult<(), Self> {
        let tp_copy = tp.get_owned();
        let mut valid = true;
        tp.actions(|action| {
            if let TracepointAction::Registers { mask: _ } = action {
                // we only handle register collection actions for the simple case
            } else {
                valid = false;
            }
        });
        if !valid {
            return Err(TargetError::NonFatal)
        }
        self.tracepoints.get_mut(&tp_copy.number).map(move |existing| {
            existing.push(TracepointItem::Define(tp_copy));
            ()
        }).ok_or_else(move || TargetError::Fatal("define on non-existing tracepoint"))
    }

    fn tracepoint_status(&self, tp: Tracepoint, _addr: u32) -> TargetResult<(u64, u64), Self> {
        // We don't collect "real" trace buffer frames, so just report hit count
        // and say the number of bytes is always 0.
        // Because we don't implement "while-stepping" actions, we don't need to
        // also check that `addr` matches.
        Ok((self.traceframes.iter().filter(|frame| frame.number.0 == tp.0).count() as u64, 0))
    }

    fn tracepoint_enumerate_start(&mut self) -> TargetResult<Option<TracepointItem<'_, u32>>, Self> {
        let tracepoints: Vec<_> = self.tracepoints.iter().flat_map(|(_key, value)| {
            value.iter().map(|item| item.get_owned())
        }).collect();
        self.tracepoint_enumerate_machine = (tracepoints, 0);

        self.tracepoint_enumerate_step()
    }

    fn tracepoint_enumerate_step<'a>(
        &'a mut self,
    ) -> TargetResult<Option<TracepointItem<'a, u32>>, Self> {
        let (tracepoints, index) = &mut self.tracepoint_enumerate_machine;
        if let Some(item) = tracepoints.iter().nth(*index) {
            *index += 1;
            Ok(Some(item.get_owned()))
        } else {
            Ok(None)
        }
    }

    fn trace_buffer_configure(&mut self, _tb: TraceBuffer) -> TargetResult<(), Self> {
        // we don't collect a "real" trace buffer, so just ignore configuration attempts.
        Ok(())
    }

    fn trace_buffer_request(
        &mut self,
        _offset: u64,
        _len: usize,
        _buf: &mut [u8],
    ) -> TargetResult<Option<usize>, Self> {
        // We don't have a "real" trace buffer, so fail all raw read requests.
        Ok(None)
    }

    fn trace_experiment_status(&self) -> TargetResult<ExperimentStatus<'_>, Self> {
        // For a bare-bones example, we don't provide in-depth status explanations.
        Ok(ExperimentStatus {
            running: self.tracing,
            explanations: ManagedSlice::Owned(vec![ExperimentExplanation::Frames(self.traceframes.len())]),
        })
    }

    fn select_frame(
        &mut self,
        frame: FrameRequest<u32>,
        report: &mut dyn FnMut(FrameDescription),
    ) -> TargetResult<(), Self> {
        // For a bare-bones example, we only support `tfind <number>` and `tfind tracepoint <tpnum>`
        // style frame selection and not the more complicated ones.
        let found = match frame {
            FrameRequest::Select(n) => {
                self.traceframes.iter().nth(n as usize).map(|frame| (n, frame))
            },
            FrameRequest::Hit(tp) => {
                let start = self.selected_frame.map(|selected| selected + 1).unwrap_or(0);
                self.traceframes.get(start..).and_then(|frames| {
                    frames.iter().enumerate().filter(|(_n, frame)| {
                        frame.number == tp
                    }).map(|(n, frame)| ((start + n) as u64, frame) ).next()
                })
            },
            _ => return Err(TargetError::NonFatal),
        };
        if let Some((n, frame)) = found {
            (report)(FrameDescription::FrameNumber(Some(n)));
            (report)(FrameDescription::Hit(frame.number));
            self.selected_frame = Some(n as usize);
        } else {
            (report)(FrameDescription::FrameNumber(None));
            self.selected_frame = None;
        }
        Ok(())
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

