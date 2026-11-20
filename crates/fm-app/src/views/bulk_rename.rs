use std::fmt;

use fm_core::{
    BulkRenameNameError, BulkRenamePreviewRow, BulkRenameRowStatus, BulkRenameSort, ExtensionMode,
    InsertPosition, MatchScope, MediaComposeMode, MediaTagKind, NumberPosition, Strings,
    TextCaseMode,
};
use fm_theme::{FONT_MONO, FONT_UI, FONT_UI_BOLD, FONT_UI_SEMIBOLD, ThemePalette, styles};
use iced::Alignment::Center;
use iced::widget::{
    Space, button, column, container, mouse_area, pick_list, row, scrollable, space, text,
    text_input,
};
use iced::{Color, Element, Length, Padding, mouse};
use lucide_icons::iced::{
    icon_check, icon_pencil, icon_rotate_ccw, icon_sliders_horizontal, icon_x,
};

use crate::app::{
    App, BulkRenameExtensionKind, BulkRenameInsertKind, BulkRenameModalState, BulkRenameMsg,
    Message, effective_bulk_rename_modal_rect,
};
use crate::views::{modal_overlay_backdrop_at, sanitize_single_line};

const PREVIEW_ROW_HEIGHT: f32 = 30.0;
const PREVIEW_OVERSCAN_ROWS: usize = 6;

#[derive(Debug, Clone, Copy)]
struct LabeledOption<T: PartialEq + Copy> {
    value: T,
    label: &'static str,
}

impl<T: PartialEq + Copy> PartialEq for LabeledOption<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T: PartialEq + Copy> Eq for LabeledOption<T> {}

impl<T: PartialEq + Copy> fmt::Display for LabeledOption<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label)
    }
}

/// Overlay modal flutuante, arrastável e redimensionável de Renomeação em Lote.
pub fn bulk_rename_modal_overlay<'a>(
    app: &'a App,
    modal: &'a BulkRenameModalState,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let (x, y, modal_w, modal_h) = effective_bulk_rename_modal_rect(app, modal);
    let card = container(bulk_rename_modal_view(app, modal, palette, strings))
        .width(Length::Fixed(modal_w))
        .height(Length::Fixed(modal_h))
        .clip(true)
        .style(styles::modal_card(palette));

    modal_overlay_backdrop_at(card, x, y)
}

fn bulk_rename_modal_view<'a>(
    app: &'a App,
    modal: &'a BulkRenameModalState,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let count_badge = container(
        text(strings.format_items_count(modal.inputs.len()))
            .size(11)
            .font(FONT_MONO)
            .color(palette.accent_primary),
    )
    .padding([3, 8])
    .style(styles::badge_container(
        palette.accent_surface,
        palette.accent_primary,
        palette.accent_border,
        6.0,
    ));

    let drag_header = mouse_area(
        row![
            container(icon_pencil().size(15).color(palette.accent_primary))
                .padding(6)
                .style(move |_| styles::panel_surface(
                    palette.accent_surface,
                    palette.accent_border,
                    8.0
                )),
            text(strings.br_title)
                .size(15)
                .font(FONT_UI_SEMIBOLD)
                .color(palette.text_primary)
                .wrapping(text::Wrapping::None),
            count_badge,
            space::horizontal(),
        ]
        .spacing(10)
        .align_y(Center),
    )
    .interaction(if modal.dragging_modal.is_some() {
        mouse::Interaction::Grabbing
    } else {
        mouse::Interaction::Grab
    })
    .on_press(Message::BulkRename(BulkRenameMsg::DragStart));

    let clear_btn = button(
        row![
            icon_rotate_ccw().size(12).color(palette.text_secondary),
            text(strings.br_clear_rules)
                .size(11)
                .color(palette.text_secondary),
        ]
        .spacing(5)
        .align_y(Center),
    )
    .padding([5, 10])
    .style(styles::pill_secondary_button(palette))
    .on_press(Message::BulkRename(BulkRenameMsg::ClearRules));

    let close_btn = button(icon_x().size(14))
        .padding([5, 7])
        .style(styles::ghost_button(palette, 6.0))
        .on_press(Message::BulkRename(BulkRenameMsg::Close));

    let header = row![
        container(drag_header).width(Length::Fill),
        clear_btn,
        close_btn,
    ]
    .spacing(8)
    .align_y(Center);

    let rules_panel = rules_column_view(modal, palette, strings);
    let preview_panel = preview_column_view(modal, palette, strings);

    let body = row![
        container(rules_panel)
            .width(Length::FillPortion(46))
            .height(Length::Fill),
        container(preview_panel)
            .width(Length::FillPortion(54))
            .height(Length::Fill),
    ]
    .spacing(12)
    .width(Length::Fill)
    .height(Length::Fill);

    let footer = footer_bar_view(app, modal, palette, strings);

    container(
        column![
            header,
            container(Space::new().width(Length::Fill).height(1))
                .style(styles::divider_line(palette)),
            body,
            container(Space::new().width(Length::Fill).height(1))
                .style(styles::divider_line(palette)),
            footer,
        ]
        .spacing(10)
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .padding(14)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn rules_column_view<'a>(
    modal: &'a BulkRenameModalState,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let cards = column![
        text(strings.br_rules_header.to_uppercase())
            .size(10)
            .font(FONT_UI_BOLD)
            .color(palette.text_muted),
        replace_card_view(modal, palette, strings),
        remove_card_view(modal, palette, strings),
        insert_card_view(modal, palette, strings),
        case_card_view(modal, palette, strings),
        number_card_view(modal, palette, strings),
        media_card_view(modal, palette, strings),
        extension_card_view(modal, palette, strings),
    ]
    .spacing(8)
    .width(Length::Fill);

    container(
        scrollable(container(cards).padding(Padding {
            top: 2.0,
            right: 8.0,
            bottom: 4.0,
            left: 0.0,
        }))
        .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn replace_card_view<'a>(
    modal: &'a BulkRenameModalState,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let r = &modal.rules;
    let body = if r.replace_enabled {
        let mut inner = column![
            row![
                labeled_input(
                    strings.br_find_label,
                    strings.br_find_label,
                    &r.find_text,
                    |s| Message::BulkRename(BulkRenameMsg::FindChanged(s)),
                    palette,
                ),
                labeled_input(
                    strings.br_replace_label,
                    strings.br_replace_label,
                    &r.replace_with,
                    |s| Message::BulkRename(BulkRenameMsg::ReplaceWithChanged(s)),
                    palette,
                ),
            ]
            .spacing(8),
        ]
        .spacing(8);

        if !r.use_regex {
            let scope_opts = match_scope_options(strings);
            let selected_scope = scope_opts
                .iter()
                .copied()
                .find(|o| o.value == r.replace_scope);
            inner = inner.push(
                row![
                    text(strings.br_where_label)
                        .size(11)
                        .color(palette.text_secondary),
                    pick_list(scope_opts, selected_scope, |opt| {
                        Message::BulkRename(BulkRenameMsg::SetReplaceScope(opt.value))
                    })
                    .text_size(12)
                    .font(FONT_UI)
                    .padding([5, 8])
                    .width(Length::Fill)
                    .style(styles::pick_list_style(palette))
                    .menu_style(styles::dropdown_menu_style(palette)),
                ]
                .spacing(8)
                .align_y(Center),
            );
        }

        inner = inner.push(
            row![
                checkbox_toggle(
                    r.replace_case_sensitive,
                    strings.br_match_case,
                    Message::BulkRename(BulkRenameMsg::ToggleReplaceCaseSensitive),
                    palette,
                ),
                checkbox_toggle(
                    r.replace_first_only,
                    strings.br_first_only,
                    Message::BulkRename(BulkRenameMsg::ToggleReplaceFirstOnly),
                    palette,
                ),
            ]
            .spacing(10),
        );

        let adv_arrow = if modal.regex_panel_expanded || r.use_regex {
            "▾"
        } else {
            "▸"
        };
        inner = inner.push(
            button(
                row![
                    icon_sliders_horizontal().size(11).color(palette.text_muted),
                    text(format!("{adv_arrow} {}", strings.br_advanced_regex))
                        .size(11)
                        .color(palette.text_muted),
                ]
                .spacing(5)
                .align_y(Center),
            )
            .padding([3, 6])
            .style(styles::ghost_button(palette, 6.0))
            .on_press(Message::BulkRename(BulkRenameMsg::ToggleAdvancedRegexPanel)),
        );

        if modal.regex_panel_expanded || r.use_regex {
            inner = inner.push(checkbox_toggle(
                r.use_regex,
                strings.br_use_regex,
                Message::BulkRename(BulkRenameMsg::ToggleUseRegex),
                palette,
            ));
        }

        if let Some(err) = &modal.eval.regex_error {
            inner = inner.push(
                container(text(err).size(11).color(palette.status_error))
                    .padding([4, 8])
                    .width(Length::Fill)
                    .style(move |_| {
                        styles::panel_surface(palette.surface_low, palette.status_error, 6.0)
                    }),
            );
        }

        Some(inner.into())
    } else {
        None
    };

    rule_card(
        r.replace_enabled,
        strings.br_replace_title,
        Message::BulkRename(BulkRenameMsg::ToggleReplace),
        body,
        palette,
    )
}

fn remove_card_view<'a>(
    modal: &'a BulkRenameModalState,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let r = &modal.rules;
    let body = if r.remove_enabled {
        let scope_opts = vec![
            LabeledOption {
                value: MatchScope::Anywhere,
                label: strings.br_scope_anywhere,
            },
            LabeledOption {
                value: MatchScope::StartOfName,
                label: strings.br_scope_start_name,
            },
            LabeledOption {
                value: MatchScope::EndOfName,
                label: strings.br_scope_end_name,
            },
        ];
        let selected_scope = scope_opts
            .iter()
            .copied()
            .find(|o| o.value == r.remove_scope);

        Some(
            column![
                row![
                    labeled_input(
                        strings.br_remove_first,
                        "0",
                        &modal.remove_first_draft,
                        |s| Message::BulkRename(BulkRenameMsg::RemoveFirstChanged(s)),
                        palette,
                    ),
                    labeled_input(
                        strings.br_remove_last,
                        "0",
                        &modal.remove_last_draft,
                        |s| Message::BulkRename(BulkRenameMsg::RemoveLastChanged(s)),
                        palette,
                    ),
                ]
                .spacing(8),
                row![
                    labeled_input(
                        strings.br_remove_text_label,
                        strings.br_remove_text_label,
                        &r.remove_text,
                        |s| Message::BulkRename(BulkRenameMsg::RemoveTextChanged(s)),
                        palette,
                    ),
                    column![
                        text(strings.br_where_label)
                            .size(11)
                            .color(palette.text_muted),
                        pick_list(scope_opts, selected_scope, |opt| {
                            Message::BulkRename(BulkRenameMsg::SetRemoveScope(opt.value))
                        })
                        .text_size(12)
                        .font(FONT_UI)
                        .padding([5, 8])
                        .width(Length::Fill)
                        .style(styles::pick_list_style(palette))
                        .menu_style(styles::dropdown_menu_style(palette)),
                    ]
                    .spacing(3)
                    .width(Length::Fill),
                ]
                .spacing(8),
                row![
                    checkbox_toggle(
                        r.remove_digits,
                        strings.br_remove_digits,
                        Message::BulkRename(BulkRenameMsg::ToggleRemoveDigits),
                        palette,
                    ),
                    checkbox_toggle(
                        r.remove_symbols,
                        strings.br_remove_symbols,
                        Message::BulkRename(BulkRenameMsg::ToggleRemoveSymbols),
                        palette,
                    ),
                    checkbox_toggle(
                        r.collapse_spaces,
                        strings.br_collapse_spaces,
                        Message::BulkRename(BulkRenameMsg::ToggleCollapseSpaces),
                        palette,
                    ),
                ]
                .spacing(8),
            ]
            .spacing(8)
            .into(),
        )
    } else {
        None
    };

    rule_card(
        r.remove_enabled,
        strings.br_remove_title,
        Message::BulkRename(BulkRenameMsg::ToggleRemove),
        body,
        palette,
    )
}

fn insert_card_view<'a>(
    modal: &'a BulkRenameModalState,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let r = &modal.rules;
    let body = if r.insert_enabled {
        let pos_opts = vec![
            LabeledOption {
                value: BulkRenameInsertKind::StartOfName,
                label: strings.br_insert_start,
            },
            LabeledOption {
                value: BulkRenameInsertKind::EndOfName,
                label: strings.br_insert_end,
            },
            LabeledOption {
                value: BulkRenameInsertKind::FromStart,
                label: strings.br_insert_offset_start,
            },
            LabeledOption {
                value: BulkRenameInsertKind::FromEnd,
                label: strings.br_insert_offset_end,
            },
        ];
        let cur_kind = match r.insert_position {
            InsertPosition::StartOfName => BulkRenameInsertKind::StartOfName,
            InsertPosition::EndOfName => BulkRenameInsertKind::EndOfName,
            InsertPosition::FromStart(_) => BulkRenameInsertKind::FromStart,
            InsertPosition::FromEnd(_) => BulkRenameInsertKind::FromEnd,
        };
        let selected_pos = pos_opts.iter().copied().find(|o| o.value == cur_kind);

        let mut col = column![
            labeled_input(
                strings.br_insert_text_label,
                strings.br_insert_text_label,
                &r.insert_text,
                |s| Message::BulkRename(BulkRenameMsg::InsertTextChanged(s)),
                palette,
            ),
            row![
                column![
                    text(strings.br_insert_pos_label)
                        .size(11)
                        .color(palette.text_muted),
                    pick_list(pos_opts, selected_pos, |opt| {
                        Message::BulkRename(BulkRenameMsg::SetInsertPositionKind(opt.value))
                    })
                    .text_size(12)
                    .font(FONT_UI)
                    .padding([5, 8])
                    .width(Length::Fill)
                    .style(styles::pick_list_style(palette))
                    .menu_style(styles::dropdown_menu_style(palette)),
                ]
                .spacing(3)
                .width(Length::FillPortion(3)),
            ]
            .spacing(8),
        ]
        .spacing(8);

        if matches!(
            r.insert_position,
            InsertPosition::FromStart(_) | InsertPosition::FromEnd(_)
        ) {
            col = col.push(
                labeled_input(
                    strings.br_insert_pos_label,
                    "0",
                    &modal.insert_offset_draft,
                    |s| Message::BulkRename(BulkRenameMsg::InsertOffsetChanged(s)),
                    palette,
                )
                .width(Length::Fixed(120.0)),
            );
        }

        Some(col.into())
    } else {
        None
    };

    rule_card(
        r.insert_enabled,
        strings.br_insert_title,
        Message::BulkRename(BulkRenameMsg::ToggleInsert),
        body,
        palette,
    )
}

fn case_card_view<'a>(
    modal: &'a BulkRenameModalState,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let r = &modal.rules;
    let body = if r.case_enabled {
        let case_opts = vec![
            LabeledOption {
                value: TextCaseMode::Keep,
                label: strings.br_case_keep,
            },
            LabeledOption {
                value: TextCaseMode::Uppercase,
                label: strings.br_case_upper,
            },
            LabeledOption {
                value: TextCaseMode::Lowercase,
                label: strings.br_case_lower,
            },
            LabeledOption {
                value: TextCaseMode::TitleCaseWords,
                label: strings.br_case_title_words,
            },
            LabeledOption {
                value: TextCaseMode::SentenceCaseFirst,
                label: strings.br_case_sentence,
            },
        ];
        let selected = case_opts.iter().copied().find(|o| o.value == r.case_mode);
        Some(
            pick_list(case_opts, selected, |opt| {
                Message::BulkRename(BulkRenameMsg::SetCaseMode(opt.value))
            })
            .text_size(12)
            .font(FONT_UI)
            .padding([6, 10])
            .width(Length::Fill)
            .style(styles::pick_list_style(palette))
            .menu_style(styles::dropdown_menu_style(palette))
            .into(),
        )
    } else {
        None
    };

    rule_card(
        r.case_enabled,
        strings.br_case_title,
        Message::BulkRename(BulkRenameMsg::ToggleCase),
        body,
        palette,
    )
}

fn number_card_view<'a>(
    modal: &'a BulkRenameModalState,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let r = &modal.rules;
    let body = if r.number_enabled {
        let pos_opts = vec![
            LabeledOption {
                value: NumberPosition::Append,
                label: strings.br_number_append,
            },
            LabeledOption {
                value: NumberPosition::Prepend,
                label: strings.br_number_prepend,
            },
            LabeledOption {
                value: NumberPosition::ReplaceName,
                label: strings.br_number_replace,
            },
        ];
        let selected_pos = pos_opts
            .iter()
            .copied()
            .find(|o| o.value == r.number_position);

        let digit_opts = vec![
            LabeledOption {
                value: 1u8,
                label: "1 (1)",
            },
            LabeledOption {
                value: 2u8,
                label: "2 (01)",
            },
            LabeledOption {
                value: 3u8,
                label: "3 (001)",
            },
            LabeledOption {
                value: 4u8,
                label: "4 (0001)",
            },
            LabeledOption {
                value: 5u8,
                label: "5 (00001)",
            },
            LabeledOption {
                value: 6u8,
                label: "6 (000001)",
            },
        ];
        let selected_digits = digit_opts
            .iter()
            .copied()
            .find(|o| o.value == r.number_digits);

        let sort_opts = sort_options(strings);
        let selected_sort = sort_opts.iter().copied().find(|o| o.value == r.sort_order);

        Some(
            column![
                row![
                    column![
                        text(strings.br_insert_pos_label)
                            .size(11)
                            .color(palette.text_muted),
                        pick_list(pos_opts, selected_pos, |opt| {
                            Message::BulkRename(BulkRenameMsg::SetNumberPosition(opt.value))
                        })
                        .text_size(12)
                        .font(FONT_UI)
                        .padding([5, 8])
                        .width(Length::Fill)
                        .style(styles::pick_list_style(palette))
                        .menu_style(styles::dropdown_menu_style(palette)),
                    ]
                    .spacing(3)
                    .width(Length::FillPortion(3)),
                    labeled_input(
                        strings.br_number_sep,
                        "_",
                        &r.number_separator,
                        |s| Message::BulkRename(BulkRenameMsg::NumberSeparatorChanged(s)),
                        palette,
                    )
                    .width(Length::FillPortion(1)),
                ]
                .spacing(8),
                row![
                    labeled_input(
                        strings.br_number_start,
                        "1",
                        &modal.number_start_draft,
                        |s| Message::BulkRename(BulkRenameMsg::NumberStartChanged(s)),
                        palette,
                    ),
                    labeled_input(
                        strings.br_number_step,
                        "1",
                        &modal.number_step_draft,
                        |s| Message::BulkRename(BulkRenameMsg::NumberStepChanged(s)),
                        palette,
                    ),
                    column![
                        text(strings.br_number_digits)
                            .size(11)
                            .color(palette.text_muted),
                        pick_list(digit_opts, selected_digits, |opt| {
                            Message::BulkRename(BulkRenameMsg::NumberDigitsChanged(opt.value))
                        })
                        .text_size(12)
                        .font(FONT_UI)
                        .padding([5, 8])
                        .width(Length::Fill)
                        .style(styles::pick_list_style(palette))
                        .menu_style(styles::dropdown_menu_style(palette)),
                    ]
                    .spacing(3)
                    .width(Length::Fill),
                ]
                .spacing(8),
                row![
                    text(strings.br_sort_label)
                        .size(11)
                        .color(palette.text_secondary),
                    pick_list(sort_opts, selected_sort, |opt| {
                        Message::BulkRename(BulkRenameMsg::SetSort(opt.value))
                    })
                    .text_size(12)
                    .font(FONT_UI)
                    .padding([5, 8])
                    .width(Length::Fill)
                    .style(styles::pick_list_style(palette))
                    .menu_style(styles::dropdown_menu_style(palette)),
                ]
                .spacing(8)
                .align_y(Center),
            ]
            .spacing(8)
            .into(),
        )
    } else {
        None
    };

    rule_card(
        r.number_enabled,
        strings.br_number_title,
        Message::BulkRename(BulkRenameMsg::ToggleNumber),
        body,
        palette,
    )
}

fn media_card_view<'a>(
    modal: &'a BulkRenameModalState,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let r = &modal.rules;
    let body = if r.media_enabled {
        let mode_opts = vec![
            LabeledOption {
                value: MediaComposeMode::PrependToName,
                label: strings.br_number_prepend,
            },
            LabeledOption {
                value: MediaComposeMode::AppendToName,
                label: strings.br_number_append,
            },
            LabeledOption {
                value: MediaComposeMode::ReplaceName,
                label: strings.br_number_replace,
            },
        ];
        let selected_mode = mode_opts.iter().copied().find(|o| o.value == r.media_mode);

        let tag_chip = |kind: MediaTagKind, label: &'a str| {
            let active = r.media_fields.contains(&kind);
            button(
                row![
                    checkbox_box(active, palette),
                    text(label)
                        .size(11)
                        .font(if active { FONT_UI_SEMIBOLD } else { FONT_UI })
                        .color(if active {
                            palette.accent_primary
                        } else {
                            palette.text_primary
                        }),
                ]
                .spacing(5)
                .align_y(Center),
            )
            .padding([5, 8])
            .style(styles::tab_button(palette, active))
            .on_press(Message::BulkRename(BulkRenameMsg::ToggleMediaField(kind)))
        };

        let mut col = column![
            row![
                column![
                    text(strings.br_insert_pos_label)
                        .size(11)
                        .color(palette.text_muted),
                    pick_list(mode_opts, selected_mode, |opt| {
                        Message::BulkRename(BulkRenameMsg::SetMediaMode(opt.value))
                    })
                    .text_size(12)
                    .font(FONT_UI)
                    .padding([5, 8])
                    .width(Length::Fill)
                    .style(styles::pick_list_style(palette))
                    .menu_style(styles::dropdown_menu_style(palette)),
                ]
                .spacing(3)
                .width(Length::FillPortion(3)),
                labeled_input(
                    strings.br_number_sep,
                    " - ",
                    &r.media_separator,
                    |s| Message::BulkRename(BulkRenameMsg::MediaSeparatorChanged(s)),
                    palette,
                )
                .width(Length::FillPortion(1)),
            ]
            .spacing(8),
            row![
                tag_chip(MediaTagKind::ExifDateYmd, strings.br_media_exif_ymd),
                tag_chip(MediaTagKind::ExifDateTime, strings.br_media_exif_datetime),
            ]
            .spacing(6),
            row![
                tag_chip(MediaTagKind::ExifCamera, strings.br_media_exif_camera),
                tag_chip(MediaTagKind::Id3Track, strings.br_media_id3_track),
            ]
            .spacing(6),
            row![
                tag_chip(MediaTagKind::Id3Artist, strings.br_media_id3_artist),
                tag_chip(MediaTagKind::Id3Album, strings.br_media_id3_album),
            ]
            .spacing(6),
            row![
                tag_chip(MediaTagKind::Id3Title, strings.br_media_id3_title),
                tag_chip(MediaTagKind::Id3Year, strings.br_media_id3_year),
            ]
            .spacing(6),
        ]
        .spacing(6);

        if modal.tags_loading {
            col = col.push(
                text(format!(
                    "{} ({}/{})",
                    strings.br_media_loading,
                    modal.tags_loaded_count,
                    modal.inputs.len()
                ))
                .size(11)
                .font(FONT_MONO)
                .color(palette.accent_primary),
            );
        }

        Some(col.into())
    } else {
        None
    };

    rule_card(
        r.media_enabled,
        strings.br_media_title,
        Message::BulkRename(BulkRenameMsg::ToggleMedia),
        body,
        palette,
    )
}

fn extension_card_view<'a>(
    modal: &'a BulkRenameModalState,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let r = &modal.rules;
    let body = if r.ext_enabled {
        let ext_opts = vec![
            LabeledOption {
                value: BulkRenameExtensionKind::Keep,
                label: strings.br_ext_keep,
            },
            LabeledOption {
                value: BulkRenameExtensionKind::Lowercase,
                label: strings.br_ext_lower,
            },
            LabeledOption {
                value: BulkRenameExtensionKind::Uppercase,
                label: strings.br_ext_upper,
            },
            LabeledOption {
                value: BulkRenameExtensionKind::Replace,
                label: strings.br_ext_replace,
            },
            LabeledOption {
                value: BulkRenameExtensionKind::Remove,
                label: strings.br_ext_remove,
            },
        ];
        let cur_kind = match &r.ext_mode {
            ExtensionMode::Keep => BulkRenameExtensionKind::Keep,
            ExtensionMode::Lowercase => BulkRenameExtensionKind::Lowercase,
            ExtensionMode::Uppercase => BulkRenameExtensionKind::Uppercase,
            ExtensionMode::Replace(_) => BulkRenameExtensionKind::Replace,
            ExtensionMode::Remove => BulkRenameExtensionKind::Remove,
        };
        let selected = ext_opts.iter().copied().find(|o| o.value == cur_kind);

        let mut col = column![
            pick_list(ext_opts, selected, |opt| {
                Message::BulkRename(BulkRenameMsg::SetExtKind(opt.value))
            })
            .text_size(12)
            .font(FONT_UI)
            .padding([6, 10])
            .width(Length::Fill)
            .style(styles::pick_list_style(palette))
            .menu_style(styles::dropdown_menu_style(palette)),
        ]
        .spacing(8);

        if cur_kind == BulkRenameExtensionKind::Replace {
            col = col.push(labeled_input(
                strings.br_ext_replace,
                "jpg",
                &modal.ext_replace_draft,
                |s| Message::BulkRename(BulkRenameMsg::ExtReplaceChanged(s)),
                palette,
            ));
        }

        Some(col.into())
    } else {
        None
    };

    rule_card(
        r.ext_enabled,
        strings.br_ext_title,
        Message::BulkRename(BulkRenameMsg::ToggleExt),
        body,
        palette,
    )
}

fn preview_column_view<'a>(
    modal: &'a BulkRenameModalState,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let header_bar = container(
        row![
            container(
                text(strings.br_col_status)
                    .size(10)
                    .font(FONT_UI_BOLD)
                    .color(palette.text_muted),
            )
            .width(Length::Fixed(52.0)),
            container(
                text(strings.br_col_original)
                    .size(10)
                    .font(FONT_UI_BOLD)
                    .color(palette.text_muted),
            )
            .width(Length::FillPortion(1)),
            container(
                text(strings.br_col_new)
                    .size(10)
                    .font(FONT_UI_BOLD)
                    .color(palette.text_muted),
            )
            .width(Length::FillPortion(1)),
        ]
        .spacing(8),
    )
    .padding([6, 10])
    .width(Length::Fill)
    .style(move |_| styles::panel_surface(palette.surface_high, palette.border, 8.0));

    let rows = &modal.eval.rows;
    let total = rows.len();
    let viewport_h = modal.preview_viewport_height.max(120.0);
    let scroll_y = modal.preview_scroll_y.max(0.0);
    let first_visible = ((scroll_y / PREVIEW_ROW_HEIGHT).floor() as usize)
        .saturating_sub(PREVIEW_OVERSCAN_ROWS)
        .min(total);
    let visible_count =
        ((viewport_h / PREVIEW_ROW_HEIGHT).ceil() as usize) + (PREVIEW_OVERSCAN_ROWS * 2) + 2;
    let end_visible = (first_visible + visible_count).min(total);

    let top_pad = (first_visible as f32) * PREVIEW_ROW_HEIGHT;
    let bottom_pad = ((total.saturating_sub(end_visible)) as f32) * PREVIEW_ROW_HEIGHT;

    let mut list_col = column![].width(Length::Fill);
    if top_pad > 0.0 {
        list_col = list_col.push(Space::new().width(Length::Fill).height(top_pad));
    }
    for row_data in &rows[first_visible..end_visible] {
        list_col = list_col.push(preview_row_item(row_data, palette, strings));
    }
    if bottom_pad > 0.0 {
        list_col = list_col.push(Space::new().width(Length::Fill).height(bottom_pad));
    }

    let scroll = scrollable(list_col)
        .width(Length::Fill)
        .height(Length::Fill)
        .on_scroll(|vp| {
            Message::BulkRename(BulkRenameMsg::PreviewScrolled {
                offset_y: vp.absolute_offset().y,
                viewport_height: vp.bounds().height,
            })
        });

    container(
        column![
            row![
                text(strings.br_preview_header.to_uppercase())
                    .size(10)
                    .font(FONT_UI_BOLD)
                    .color(palette.text_muted),
                space::horizontal(),
            ],
            header_bar,
            container(scroll)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(move |_| {
                    styles::panel_surface(palette.surface_low, palette.border, 8.0)
                }),
        ]
        .spacing(6)
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn preview_row_item<'a>(
    row_data: &'a BulkRenamePreviewRow,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let orig_display = sanitize_single_line(&row_data.original_name.to_string_lossy());
    let new_display = sanitize_single_line(&row_data.new_name.to_string_lossy());

    let (badge_label, badge_fg, badge_bg, new_text, new_color) = match row_data.status {
        BulkRenameRowStatus::Ready => (
            "OK",
            palette.accent_primary,
            palette.accent_surface,
            new_display,
            palette.accent_primary,
        ),
        BulkRenameRowStatus::Unchanged => (
            "--",
            palette.text_muted,
            palette.surface_high,
            strings.br_unchanged.to_owned(),
            palette.text_muted,
        ),
        BulkRenameRowStatus::SkippedFolder => (
            "--",
            palette.text_muted,
            palette.surface_high,
            strings.br_skipped_folder.to_owned(),
            palette.text_muted,
        ),
        BulkRenameRowStatus::ConflictInternal => (
            "!!",
            palette.status_error,
            palette.surface_high,
            format!("{new_display} — {}", strings.br_conflict_internal),
            palette.status_error,
        ),
        BulkRenameRowStatus::ConflictExternal => (
            "!!",
            palette.status_error,
            palette.surface_high,
            format!("{new_display} — {}", strings.br_conflict_external),
            palette.status_error,
        ),
        BulkRenameRowStatus::InvalidName(err) => {
            let reason = match err {
                BulkRenameNameError::Empty => strings.br_invalid_name,
                BulkRenameNameError::ReservedDot => strings.br_invalid_name,
                BulkRenameNameError::ContainsPathSeparator => strings.br_invalid_name,
                BulkRenameNameError::WindowsInvalidChar => strings.br_invalid_name,
                BulkRenameNameError::WindowsReservedName => strings.br_invalid_name,
                BulkRenameNameError::TooLong => strings.br_invalid_name,
            };
            (
                "!!",
                palette.status_error,
                palette.surface_high,
                format!("{new_display} — {reason}"),
                palette.status_error,
            )
        }
    };

    let status_pill = container(text(badge_label).size(10).font(FONT_MONO).color(badge_fg))
        .width(Length::Fixed(42.0))
        .padding([2, 6])
        .align_x(Center)
        .style(styles::badge_container(badge_bg, badge_fg, badge_fg, 6.0));

    container(
        row![
            container(status_pill).width(Length::Fixed(52.0)),
            container(
                text(orig_display)
                    .size(12)
                    .font(FONT_UI)
                    .color(palette.text_secondary)
                    .wrapping(text::Wrapping::None),
            )
            .width(Length::FillPortion(1))
            .clip(true),
            container(
                text(new_text)
                    .size(12)
                    .font(if row_data.status == BulkRenameRowStatus::Ready {
                        FONT_UI_SEMIBOLD
                    } else {
                        FONT_UI
                    })
                    .color(new_color)
                    .wrapping(text::Wrapping::None),
            )
            .width(Length::FillPortion(1))
            .clip(true),
        ]
        .spacing(8)
        .align_y(Center),
    )
    .height(Length::Fixed(PREVIEW_ROW_HEIGHT))
    .padding([2, 10])
    .width(Length::Fill)
    .into()
}

fn footer_bar_view<'a>(
    app: &'a App,
    modal: &'a BulkRenameModalState,
    palette: ThemePalette,
    strings: Strings,
) -> Element<'a, Message> {
    let ev = &modal.eval;
    let summary_text = if ev.conflict_count > 0 || ev.invalid_count > 0 {
        format!(
            "{} ready · {} · {}!",
            ev.ready_count,
            ev.unchanged_count,
            ev.conflict_count + ev.invalid_count
        )
    } else {
        format!(
            "{} ready · {}",
            ev.ready_count,
            ev.unchanged_count + ev.skipped_count
        )
    };
    let summary_color = if ev.conflict_count > 0 || ev.invalid_count > 0 || ev.regex_error.is_some()
    {
        palette.status_error
    } else if ev.ready_count > 0 {
        palette.accent_primary
    } else {
        palette.text_muted
    };

    let include_folders_chk = checkbox_toggle(
        modal.rules.include_folders,
        strings.br_include_folders,
        Message::BulkRename(BulkRenameMsg::ToggleIncludeFolders),
        palette,
    );

    let mut action_row = row![].spacing(8).align_y(Center);

    if app.last_bulk_rename_undo.is_some() && !modal.applying {
        action_row = action_row.push(
            button(
                row![
                    icon_rotate_ccw().size(12),
                    text(strings.br_undo_last).size(12),
                ]
                .spacing(5)
                .align_y(Center),
            )
            .padding([6, 12])
            .style(styles::pill_secondary_button(palette))
            .on_press(Message::UndoLastBulkRename),
        );
    }

    action_row = action_row.push(
        button(text(strings.cancel).size(12))
            .padding([6, 14])
            .style(styles::pill_secondary_button(palette))
            .on_press(Message::BulkRename(BulkRenameMsg::Close)),
    );

    let apply_label = if modal.applying {
        strings.br_applying.to_owned()
    } else {
        format!("{} ({})", strings.br_apply, ev.ready_count)
    };
    let mut apply_btn = button(text(apply_label).size(12).font(FONT_UI_SEMIBOLD))
        .padding([6, 16])
        .style(styles::pill_primary_button(palette));
    if ev.can_apply() && !modal.applying {
        apply_btn = apply_btn.on_press(Message::BulkRename(BulkRenameMsg::Apply));
    }
    action_row = action_row.push(apply_btn);

    let grip_dot = |p: ThemePalette| {
        container(Space::new().width(3).height(3))
            .style(move |_| styles::panel_surface(p.text_muted, p.text_muted, 1.5))
    };
    let resize_grip = mouse_area(
        container(
            column![
                row![
                    Space::new().width(2).height(2),
                    Space::new().width(2).height(2),
                    grip_dot(palette),
                ]
                .spacing(2),
                row![
                    Space::new().width(2).height(2),
                    grip_dot(palette),
                    grip_dot(palette),
                ]
                .spacing(2),
                row![grip_dot(palette), grip_dot(palette), grip_dot(palette)].spacing(2),
            ]
            .spacing(2),
        )
        .padding([4, 4]),
    )
    .interaction(mouse::Interaction::ResizingDiagonallyDown)
    .on_press(Message::BulkRename(BulkRenameMsg::ResizeStart));

    row![
        include_folders_chk,
        container(
            text(summary_text)
                .size(11)
                .font(FONT_MONO)
                .color(summary_color)
        )
        .padding([3, 8])
        .style(move |_| styles::panel_surface(palette.surface_low, palette.border, 6.0)),
        space::horizontal(),
        action_row,
        resize_grip,
    ]
    .spacing(10)
    .align_y(Center)
    .into()
}

fn rule_card<'a>(
    enabled: bool,
    title: &'a str,
    on_toggle: Message,
    body: Option<Element<'a, Message>>,
    palette: ThemePalette,
) -> Element<'a, Message> {
    let header_btn = button(
        row![
            checkbox_box(enabled, palette),
            text(title)
                .size(12)
                .font(FONT_UI_SEMIBOLD)
                .color(if enabled {
                    palette.text_primary
                } else {
                    palette.text_secondary
                }),
            space::horizontal(),
        ]
        .spacing(8)
        .align_y(Center),
    )
    .width(Length::Fill)
    .padding([4, 2])
    .style(styles::ghost_button(palette, 6.0))
    .on_press(on_toggle);

    let mut content = column![header_btn].spacing(8).width(Length::Fill);
    if let Some(inner) = body {
        content = content.push(inner);
    }

    container(content)
        .padding([8, 10])
        .width(Length::Fill)
        .style(move |_| {
            let border = if enabled {
                palette.accent_border
            } else {
                palette.border
            };
            styles::panel_surface(palette.surface_low, border, 8.0)
        })
        .into()
}

fn labeled_input<'a>(
    label: &'a str,
    placeholder: &'a str,
    value: &'a str,
    on_change: impl Fn(String) -> Message + 'a,
    palette: ThemePalette,
) -> iced::widget::Column<'a, Message> {
    column![
        text(label).size(11).color(palette.text_muted),
        container(
            text_input(placeholder, value)
                .on_input(on_change)
                .size(12)
                .padding([5, 8])
                .style(styles::search_input_style(palette)),
        )
        .padding([1, 4])
        .width(Length::Fill)
        .style(move |_| styles::panel_surface(palette.surface_lowest, palette.border, 6.0)),
    ]
    .spacing(3)
    .width(Length::Fill)
}

fn checkbox_toggle<'a>(
    checked: bool,
    label: &'a str,
    on_press: Message,
    palette: ThemePalette,
) -> Element<'a, Message> {
    button(
        row![
            checkbox_box(checked, palette),
            text(label)
                .size(11)
                .font(FONT_UI)
                .color(palette.text_primary),
        ]
        .spacing(6)
        .align_y(Center),
    )
    .padding([4, 6])
    .style(styles::ghost_button(palette, 6.0))
    .on_press(on_press)
    .into()
}

fn checkbox_box<'a>(active: bool, palette: ThemePalette) -> Element<'a, Message> {
    let fill = if active {
        palette.accent_primary
    } else {
        palette.surface_high
    };
    let border = if active {
        palette.accent_primary
    } else {
        palette.border_strong
    };
    let fg: Color = palette.on_accent;
    container(if active {
        Element::from(icon_check().size(11).color(fg))
    } else {
        Space::new().width(11).height(11).into()
    })
    .width(Length::Fixed(16.0))
    .height(Length::Fixed(16.0))
    .align_x(Center)
    .align_y(Center)
    .style(move |_| styles::panel_surface(fill, border, 4.0))
    .into()
}

fn match_scope_options(strings: Strings) -> Vec<LabeledOption<MatchScope>> {
    vec![
        LabeledOption {
            value: MatchScope::Anywhere,
            label: strings.br_scope_anywhere,
        },
        LabeledOption {
            value: MatchScope::StartOfName,
            label: strings.br_scope_start_name,
        },
        LabeledOption {
            value: MatchScope::EndOfName,
            label: strings.br_scope_end_name,
        },
        LabeledOption {
            value: MatchScope::WordStart,
            label: strings.br_scope_word_start,
        },
        LabeledOption {
            value: MatchScope::WordEnd,
            label: strings.br_scope_word_end,
        },
        LabeledOption {
            value: MatchScope::WholeWord,
            label: strings.br_scope_whole_word,
        },
    ]
}

fn sort_options(strings: Strings) -> Vec<LabeledOption<BulkRenameSort>> {
    vec![
        LabeledOption {
            value: BulkRenameSort::SelectionOrder,
            label: strings.br_sort_selection,
        },
        LabeledOption {
            value: BulkRenameSort::NameAsc,
            label: strings.br_sort_name_asc,
        },
        LabeledOption {
            value: BulkRenameSort::NameDesc,
            label: strings.br_sort_name_desc,
        },
        LabeledOption {
            value: BulkRenameSort::ModifiedDesc,
            label: strings.br_sort_date_desc,
        },
        LabeledOption {
            value: BulkRenameSort::ModifiedAsc,
            label: strings.br_sort_date_asc,
        },
        LabeledOption {
            value: BulkRenameSort::SizeAsc,
            label: strings.br_sort_size_asc,
        },
        LabeledOption {
            value: BulkRenameSort::SizeDesc,
            label: strings.br_sort_size_desc,
        },
    ]
}
