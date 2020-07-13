use {
    super::*,
    crate::{
        command::{Command, TriggerType},
        display::{Screen, W},
        errors::ProgramError,
        flag::Flag,
        pattern::*,
        selection_type::SelectionType,
        skin::PanelSkin,
        task_sync::Dam,
        verb::*,
    },
    std::path::{Path, PathBuf},
    termimad::Area,
};

/// a whole application state, stackable to allow reverting
///  to a previous one
pub trait AppState {
    /// called on start of on_command
    fn clear_pending(&mut self) {}

    fn on_click(
        &mut self,
        _x: u16,
        _y: u16,
        _screen: &mut Screen,
        _con: &AppContext,
    ) -> Result<AppStateCmdResult, ProgramError> {
        Ok(AppStateCmdResult::Keep)
    }

    fn on_double_click(
        &mut self,
        _x: u16,
        _y: u16,
        _screen: &mut Screen,
        _con: &AppContext,
    ) -> Result<AppStateCmdResult, ProgramError> {
        Ok(AppStateCmdResult::Keep)
    }

    fn on_pattern(
        &mut self,
        _pat: InputPattern,
        _con: &AppContext,
    ) -> Result<AppStateCmdResult, ProgramError> {
        Ok(AppStateCmdResult::Keep)
    }

    /// execute the internal with the optional given invocation.
    ///
    /// The invocation comes from the input and may be related
    /// to a different verb (the verb may have been triggered
    /// by a key shorctut)
    fn on_internal(
        &mut self,
        w: &mut W,
        internal_exec: &InternalExecution,
        input_invocation: Option<&VerbInvocation>,
        trigger_type: TriggerType,
        cc: &CmdContext,
        screen: &mut Screen,
    ) -> Result<AppStateCmdResult, ProgramError>;

    /// change the state, does no rendering
    fn on_command(
        &mut self,
        w: &mut W,
        cc: &CmdContext,
        screen: &mut Screen,
    ) -> Result<AppStateCmdResult, ProgramError> {
        self.clear_pending();
        let con = &cc.con;
        match cc.cmd {
            Command::Click(x, y) => self.on_click(*x, *y, screen, con),
            Command::DoubleClick(x, y) => self.on_double_click(*x, *y, screen, con),
            Command::PatternEdit { raw, expr } => {
                match InputPattern::new(raw.clone(), expr, &cc.con) {
                    Ok(pattern) => self.on_pattern(pattern, con),
                    Err(e) => Ok(AppStateCmdResult::DisplayError(format!("{}", e))),
                }
            }
            Command::VerbTrigger {
                index,
                input_invocation,
            } => {
                let verb = &con.verb_store.verbs[*index];
                match &verb.execution {
                    VerbExecution::Internal(internal_exec) => self.on_internal(
                        w,
                        internal_exec,
                        input_invocation.as_ref(),
                        TriggerType::Other,
                        cc,
                        screen,
                    ),
                    VerbExecution::External(external) => external.to_cmd_result(
                        w,
                        self.selected_path(),
                        &cc.other_path,
                        if let Some(inv) = &input_invocation {
                            &inv.args
                        } else {
                            &None
                        },
                        con,
                    ),
                }
            }
            Command::Internal {
                internal,
                input_invocation,
            } => self.on_internal(
                w,
                &InternalExecution::from_internal(*internal),
                input_invocation.as_ref(),
                TriggerType::Other,
                cc,
                screen,
            ),
            Command::VerbInvocate(invocation) => match con.verb_store.search(&invocation.name) {
                PrefixSearchResult::Match(_, verb) => {
                    if let Some(err) = verb.check_args(invocation, &cc.other_path) {
                        Ok(AppStateCmdResult::DisplayError(err))
                    } else {
                        match &verb.execution {
                            VerbExecution::Internal(internal_exec) => self.on_internal(
                                w,
                                internal_exec,
                                Some(invocation),
                                TriggerType::Input,
                                cc,
                                screen,
                            ),
                            VerbExecution::External(external) => {
                                external.to_cmd_result(
                                    w,
                                    self.selected_path(),
                                    &cc.other_path,
                                    &invocation.args,
                                    con,
                                )
                            }
                        }
                    }
                }
                _ => Ok(AppStateCmdResult::verb_not_found(&invocation.name)),
            },
            Command::None | Command::VerbEdit(_) => {
                // we do nothing here, the real job is done in get_status
                Ok(AppStateCmdResult::Keep)
            }
        }
    }

    fn selected_path(&self) -> &Path;
    fn selection_type(&self) -> SelectionType;

    fn refresh(&mut self, screen: &Screen, con: &AppContext) -> Command;

    fn do_pending_task(
        &mut self,
        _screen: &mut Screen,
        _con: &AppContext,
        _dam: &mut Dam,
    ) {
        // no pending task in default impl
        unreachable!();
    }

    fn get_pending_task(&self) -> Option<&'static str> {
        None
    }

    fn display(
        &mut self,
        w: &mut W,
        screen: &Screen,
        state_area: Area,
        skin: &PanelSkin,
        con: &AppContext,
    ) -> Result<(), ProgramError>;

    fn get_status(
        &self,
        cmd: &Command,
        other_path: &Option<PathBuf>,
        con: &AppContext,
    ) -> Status;

    /// return the flags to display
    fn get_flags(&self) -> Vec<Flag> {
        vec![]
    }

    fn get_starting_input(&self) -> String {
        String::new()
    }

    fn set_selected_path(&mut self, _path: PathBuf) {
        unreachable!(); // is_file_preview is tested before
    }
}
