use crate::terminal::PluralTerm;
use crate::{color, fmt_info, fmt_warn, OckamColor, Terminal, TerminalStream};
use colorful::Colorful;
use console::Term;
use miette::{miette, IntoDiagnostic};

use super::color_primary;

#[ockam_core::async_trait]
pub trait ShowCommandTui {
    const ITEM_NAME: PluralTerm;

    fn cmd_arg_item_name(&self) -> Option<&str>;
    fn node_name(&self) -> Option<&str> {
        None
    }
    fn terminal(&self) -> Terminal<TerminalStream<Term>>;

    async fn get_arg_item_name_or_default(&self) -> miette::Result<String>;
    async fn list_items_names(&self) -> miette::Result<Vec<String>>;
    async fn show_single(&self, item_name: &str) -> miette::Result<()>;

    async fn show(&self) -> miette::Result<()> {
        let terminal = self.terminal();
        let items_names = self.list_items_names().await?;
        if items_names.is_empty() {
            terminal
                .stdout()
                .plain(fmt_info!(
                    "There are no {} to show{}",
                    Self::ITEM_NAME.plural(),
                    get_opt_node_name_message(self.node_name())
                ))
                .json(serde_json::to_string(&items_names).into_diagnostic()?)
                .write_line()?;
            return Ok(());
        }

        if self.cmd_arg_item_name().is_some() || !terminal.can_ask_for_user_input() {
            let item_name = self.get_arg_item_name_or_default().await?;
            if !items_names.contains(&item_name) {
                return Err(miette!(
                    "The {} {} was not found",
                    Self::ITEM_NAME.singular(),
                    color!(item_name, OckamColor::PrimaryResource)
                ));
            }
            self.show_single(&item_name).await?;
            return Ok(());
        }

        match items_names.len() {
            0 => {
                unreachable!("this case is already handled above");
            }
            1 => {
                let item_name = items_names[0].as_str();
                self.show_single(item_name).await?;
            }
            _ => {
                let selected_item_names = terminal.select_multiple(
                    format!(
                        "Select one or more {} that you want to show:",
                        Self::ITEM_NAME.plural()
                    ),
                    items_names,
                );
                match selected_item_names.len() {
                    0 => {
                        terminal
                            .stdout()
                            .plain(fmt_info!(
                                "No {} selected to show",
                                Self::ITEM_NAME.plural()
                            ))
                            .write_line()?;
                    }
                    1 => {
                        let item_name = selected_item_names[0].as_str();
                        self.show_single(item_name).await?;
                    }
                    _ => {
                        for item_name in selected_item_names {
                            if self.show_single(&item_name).await.is_err() {
                                self.terminal()
                                    .stdout()
                                    .plain(fmt_warn!(
                                        "Failed to show {} {}",
                                        Self::ITEM_NAME.singular(),
                                        color_primary(item_name)
                                    ))
                                    .write_line()?;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

pub fn get_opt_node_name_message(node_name: Option<&str>) -> String {
    if let Some(node_name) = node_name {
        format!(" on node {}", color_primary(node_name))
    } else {
        "".to_string()
    }
}

#[ockam_core::async_trait]
pub trait DeleteCommandTui {
    const ITEM_NAME: PluralTerm;

    fn cmd_arg_item_name(&self) -> Option<&str>;
    fn cmd_arg_delete_all(&self) -> bool;
    fn cmd_arg_confirm_deletion(&self) -> bool;
    fn terminal(&self) -> Terminal<TerminalStream<Term>>;

    async fn get_arg_item_name_or_default(&self) -> miette::Result<String>;
    async fn list_items_names(&self) -> miette::Result<Vec<String>>;
    async fn delete_single(&self, item_name: &str) -> miette::Result<()>;
    async fn delete_multiple(&self, items_names: Vec<String>) -> miette::Result<()> {
        for item_name in items_names {
            if self.delete_single(&item_name).await.is_err() {
                self.terminal()
                    .stdout()
                    .plain(fmt_warn!(
                        "Failed to delete {} {}",
                        Self::ITEM_NAME.singular(),
                        color!(item_name, OckamColor::PrimaryResource)
                    ))
                    .write_line()?;
            }
        }
        Ok(())
    }

    async fn delete(&self) -> miette::Result<()> {
        let terminal = self.terminal();
        let items_names = self.list_items_names().await?;

        if items_names.is_empty() {
            terminal
                .stdout()
                .plain(fmt_info!(
                    "There are no {} to delete",
                    Self::ITEM_NAME.plural()
                ))
                .json(serde_json::to_string(&items_names).into_diagnostic()?)
                .write_line()?;
            return Ok(());
        }

        if self.cmd_arg_delete_all()
            && terminal.confirmed_with_flag_or_prompt(
                self.cmd_arg_confirm_deletion(),
                format!(
                    "Are you sure you want to delete {}?",
                    if items_names.len() > 1 {
                        format!("your {} {}", items_names.len(), Self::ITEM_NAME.plural())
                    } else {
                        format!("your only {}", Self::ITEM_NAME.singular())
                    }
                ),
            )?
        {
            self.delete_multiple(items_names).await?;
            return Ok(());
        }

        if self.cmd_arg_item_name().is_some() || !terminal.can_ask_for_user_input() {
            if let Some(item_name) = self.cmd_arg_item_name() {
                if !items_names.contains(&item_name.to_string()) {
                    return Err(miette!(
                        "The {} {} was not found",
                        Self::ITEM_NAME.singular(),
                        color_primary(item_name)
                    ));
                }
                if terminal.confirmed_with_flag_or_prompt(
                    self.cmd_arg_confirm_deletion(),
                    "Are you sure you want to proceed?",
                )? {
                    self.delete_single(item_name).await?;
                }
            }
            return Ok(());
        }

        match items_names.len() {
            0 => {
                unreachable!("this case is already handled above");
            }
            1 => {
                if terminal.confirmed_with_flag_or_prompt(
                    self.cmd_arg_confirm_deletion(),
                    "You are about to delete your only Outlet. Are you sure you want to proceed?",
                )? {
                    let item_name = items_names[0].as_str();
                    self.delete_single(item_name).await?;
                }
            }
            _ => {
                let selected_item_names = terminal.select_multiple(
                    format!(
                        "Select one or more {} that you want to delete:",
                        Self::ITEM_NAME.plural()
                    ),
                    items_names,
                );
                match selected_item_names.len() {
                    0 => {
                        terminal
                            .stdout()
                            .plain(fmt_info!(
                                "No {} selected to delete",
                                Self::ITEM_NAME.plural()
                            ))
                            .write_line()?;
                    }
                    1 => {
                        if terminal.confirmed_with_flag_or_prompt(
                            self.cmd_arg_confirm_deletion(),
                            "Are you sure you want to proceed?",
                        )? {
                            let item_name = selected_item_names[0].as_str();
                            self.delete_single(item_name).await?;
                        }
                    }
                    _ => {
                        if terminal.confirmed_with_flag_or_prompt(
                            self.cmd_arg_confirm_deletion(),
                            "Are you sure you want to proceed?",
                        )? {
                            self.delete_multiple(selected_item_names).await?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
