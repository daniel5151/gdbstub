use crate::emu::Emu;
use gdbstub::target;
use gdbstub::target::ext::tracepoints::ExperimentExplanation;
use gdbstub::target::ext::tracepoints::ExperimentStatus;
use gdbstub::target::ext::tracepoints::FrameDescription;
use gdbstub::target::ext::tracepoints::FrameRequest;
use gdbstub::target::ext::tracepoints::NewTracepoint;
use gdbstub::target::ext::tracepoints::SourceTracepoint;
use gdbstub::target::ext::tracepoints::TraceBufferConfig;
use gdbstub::target::ext::tracepoints::Tracepoint;
use gdbstub::target::ext::tracepoints::TracepointAction;
use gdbstub::target::ext::tracepoints::TracepointEnumerateState;
use gdbstub::target::ext::tracepoints::TracepointEnumerateStep;
use gdbstub::target::ext::tracepoints::TracepointStatus;
use gdbstub::target::TargetError;
use gdbstub::target::TargetResult;

impl Emu {
    fn step_to_next_tracepoint(&self, tp: Tracepoint) -> TracepointEnumerateStep<u32> {
        let next_tp = self.tracepoints.range(tp..).nth(1);
        if let Some((tp, (new_tp, _, _))) = next_tp {
            TracepointEnumerateStep::Next {
                tp: *tp,
                addr: new_tp.addr,
            }
        } else {
            // No more tracepoints
            TracepointEnumerateStep::Done
        }
    }
}

impl target::ext::tracepoints::Tracepoints for Emu {
    fn tracepoints_init(&mut self) -> TargetResult<(), Self> {
        self.tracepoints.clear();
        self.traceframes.clear();
        Ok(())
    }

    fn tracepoint_create_begin(&mut self, tp: NewTracepoint<u32>) -> TargetResult<(), Self> {
        self.tracepoints.insert(tp.number, (tp, vec![], vec![]));
        Ok(())
    }

    fn tracepoint_create_continue(
        &mut self,
        tp: Tracepoint,
        action: &TracepointAction<'_, u32>,
    ) -> TargetResult<(), Self> {
        if let &TracepointAction::Registers { mask: _ } = &action {
            // we only handle register collection actions for the simple
            // case
        } else {
            return Err(TargetError::NonFatal);
        }
        self.tracepoints
            .get_mut(&tp)
            .map(move |(_ctp, _source, actions)| actions.push(action.get_owned()))
            .ok_or(TargetError::Fatal("extend on non-existing tracepoint"))
    }

    fn tracepoint_create_complete(&mut self, _tp: Tracepoint) -> TargetResult<(), Self> {
        /* nothing to do */
        Ok(())
    }

    fn tracepoint_status(
        &self,
        tp: Tracepoint,
        _addr: u32,
    ) -> TargetResult<TracepointStatus, Self> {
        // We don't collect "real" trace buffer frames, so just report hit count
        // and say the number of bytes is always 0.
        // Because we don't implement "while-stepping" actions, we don't need to
        // also check that `addr` matches.
        Ok(TracepointStatus {
            hit_count: self
                .traceframes
                .iter()
                .filter(|frame| frame.number.0 == tp.0)
                .count() as u64,
            bytes_used: 0,
        })
    }

    fn tracepoint_enumerate_state(&mut self) -> &mut TracepointEnumerateState<u32> {
        &mut self.tracepoint_enumerate_state
    }

    fn tracepoint_enumerate_start(
        &mut self,
        tp: Option<Tracepoint>,
        f: &mut dyn FnMut(&NewTracepoint<u32>),
    ) -> TargetResult<TracepointEnumerateStep<u32>, Self> {
        let tp = match tp {
            Some(tp) => tp,
            None => {
                // We have no tracepoints to report
                if self.tracepoints.is_empty() {
                    return Ok(TracepointEnumerateStep::Done);
                } else {
                    // Start enumerating at the first one
                    *self.tracepoints.keys().next().unwrap()
                }
            }
        };

        // Report our tracepoint
        (f)(&self.tracepoints[&tp].0);

        let ret = if !self.tracepoints[&tp].1.is_empty() {
            TracepointEnumerateStep::Source
        } else if !self.tracepoints[&tp].2.is_empty() {
            TracepointEnumerateStep::Action
        } else {
            TracepointEnumerateStep::Done
        };

        Ok(ret)
    }

    fn tracepoint_enumerate_action(
        &mut self,
        tp: Tracepoint,
        step: u64,
        f: &mut dyn FnMut(&TracepointAction<'_, u32>),
    ) -> TargetResult<TracepointEnumerateStep<u32>, Self> {
        // Report our next action
        (f)(&self.tracepoints[&tp].2[step as usize]);

        let ret = if self.tracepoints[&tp].2.get((step as usize) + 1).is_some() {
            // Continue stepping
            TracepointEnumerateStep::Action
        } else if !self.tracepoints[&tp].1.is_empty() {
            // We're done with this tracepoint, report source
            TracepointEnumerateStep::Source
        } else {
            // No sources, move to the next tracepoint
            self.step_to_next_tracepoint(tp)
        };

        Ok(ret)
    }

    #[inline(always)]
    fn support_tracepoint_source(
        &mut self,
    ) -> Option<target::ext::tracepoints::TracepointSourceOps<'_, Self>> {
        Some(self)
    }

    fn trace_buffer_configure(&mut self, _config: TraceBufferConfig) -> TargetResult<(), Self> {
        // we don't collect a "real" trace buffer, so just ignore configuration
        // attempts.
        Ok(())
    }

    fn trace_buffer_request(
        &mut self,
        _offset: u64,
        _len: usize,
        _f: &mut dyn FnMut(&mut [u8]),
    ) -> TargetResult<(), Self> {
        // We don't have a "real" trace buffer, so just don't report any data
        Ok(())
    }

    fn trace_experiment_status(
        &self,
        report: &mut dyn FnMut(ExperimentStatus<'_>),
    ) -> TargetResult<(), Self> {
        // For a bare-bones example, we don't provide in-depth status explanations.
        (report)(if self.tracing {
            ExperimentStatus::Running
        } else {
            ExperimentStatus::NotRunning
        });
        Ok(())
    }

    fn trace_experiment_info(
        &self,
        report: &mut dyn FnMut(ExperimentExplanation<'_>),
    ) -> TargetResult<(), Self> {
        (report)(ExperimentExplanation::Frames(self.traceframes.len()));

        Ok(())
    }

    fn select_frame(
        &mut self,
        frame: FrameRequest<u32>,
        report: &mut dyn FnMut(FrameDescription),
    ) -> TargetResult<(), Self> {
        // For a bare-bones example, we only support `tfind <number>` and `tfind
        // tracepoint <tpnum>` style frame selection and not the more
        // complicated ones.
        let found = match frame {
            FrameRequest::Select(n) => self.traceframes.get(n as usize).map(|frame| (n, frame)),
            FrameRequest::Hit(tp) => {
                let start = self
                    .selected_frame
                    .map(|selected| selected + 1)
                    .unwrap_or(0);
                self.traceframes.get(start..).and_then(|frames| {
                    frames
                        .iter()
                        .enumerate()
                        .filter(|(_n, frame)| frame.number == tp)
                        .map(|(n, frame)| ((start + n) as u64, frame))
                        .next()
                })
            }
            _ => return Err(TargetError::NonFatal),
        };
        if let Some((n, frame)) = found {
            (report)(FrameDescription::FrameNumber(n));
            (report)(FrameDescription::Hit(frame.number));
            self.selected_frame = Some(n as usize);
        } else {
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

impl target::ext::tracepoints::TracepointSource for Emu {
    fn tracepoint_enumerate_source(
        &mut self,
        tp: Tracepoint,
        step: u64,
        f: &mut dyn FnMut(&SourceTracepoint<'_, u32>),
    ) -> TargetResult<TracepointEnumerateStep<u32>, Self> {
        // Report our next source item
        (f)(&self.tracepoints[&tp].1[step as usize]);

        let ret = if self.tracepoints[&tp].1.get((step as usize) + 1).is_some() {
            // Continue stepping
            TracepointEnumerateStep::Source
        } else {
            // Move to next tracepoint
            self.step_to_next_tracepoint(tp)
        };

        Ok(ret)
    }

    fn tracepoint_attach_source(
        &mut self,
        src: SourceTracepoint<'_, u32>,
    ) -> TargetResult<(), Self> {
        self.tracepoints
            .get_mut(&src.number)
            .unwrap()
            .1
            .push(src.get_owned());
        Ok(())
    }
}
