use iced::{
    Alignment, Element, Length,
    widget::{
        Column, Row, button, checkbox, column, container, pick_list, row, scrollable, space, svg,
        text, tooltip,
    },
};

use crate::gui::{Message, Status, Velvet, theme};

pub const WINDOW_HEIGHT: f32 = 275.0;

pub fn view(velvet: &Velvet) -> Column<'_, Message> {
    let (button_message, extra_message): (&str, Option<Element<Message>>) = match &velvet.status {
        Status::Idle => ("Install", None),
        Status::Installing => ("Installing...", None),
        Status::NoVersion => ("No version selected!", None),
        Status::Success(not_found) => (
            "Finished!",
            if !not_found.is_empty() {
                Some({
                    let mut mod_string = String::new();
                    mod_string.push_str(&not_found[0]);
                    for name in not_found.iter().skip(1) {
                        mod_string.push_str(", ");
                        mod_string.push_str(name);
                    }
                    tooltip(
                        "Hover to see unavailable mods.",
                        container(text(mod_string)),
                        tooltip::Position::FollowCursor,
                    )
                    .style(theme::container_style)
                    .into()
                })
            } else {
                None
            },
        ),
        Status::Failure(e) => ("Error!", Some(text(e).color(theme::LOVE).into())),
    };

    let extra_mods: Element<Message> = if velvet.extra_mods.is_empty() {
        let mut row: Row<Message> = row![];
        row = row.push(space().width(Length::Fill));
        row = row.push(text("Extra mods...").color(theme::SUBTLE));
        row = row.push(space().width(Length::Fill));
        container(row)
            .center_y(32)
            .style(theme::extra_mods_container_style)
            .into()
    } else {
        let mut row: Row<Message> = row![];
        for (index, extra_mod) in velvet.extra_mods.iter().enumerate() {
            row = row.push(
                button(
                    text(extra_mod.title.as_str())
                        .size(14)
                        .align_y(Alignment::Center),
                )
                .height(32)
                .on_press(Message::RemoveExtraMod(index))
                .style(theme::extra_mods_button_style),
            );
        }
        row = row.push(space().width(Length::Fill));
        scrollable(row)
            .direction(scrollable::Direction::Horizontal(
                scrollable::Scrollbar::new().width(4).scroller_width(4),
            ))
            .height(32)
            .style(theme::extra_mods_scrollable_style)
            .into()
    };

    let mut main_column = column![
        text("Enter Minecraft version:").size(20),
        pick_list(
            velvet.version_list.clone(),
            velvet.version.clone(),
            Message::UpdateVersion
        )
        .placeholder("Loading...")
        .width(200)
        .style(theme::pick_list_style)
        .menu_style(theme::menu_style),
        space().height(10),
        checkbox(velvet.snapshot)
            .label("Show snapshots")
            .on_toggle(Message::Snapshot)
            .style(theme::checkbox_style),
        column![
            checkbox(velvet.vanilla)
                .label("Vanilla - Performance enhancing modlist.")
                .on_toggle(Message::VButton)
                .style(theme::checkbox_style),
            checkbox(velvet.beauty)
                .label("Beauty - Immersive and beautiful modlist.")
                .on_toggle(Message::BButton)
                .style(theme::checkbox_style),
            checkbox(velvet.optifine)
                .label("Optifine - Optifine resource pack parity.")
                .on_toggle(Message::OButton)
                .style(theme::checkbox_style),
        ]
        .align_x(Alignment::Start),
        row![
            extra_mods,
            button(svg(velvet.icons.search.clone()))
                .on_press(Message::OpenExtraWindow)
                .style(theme::button_style)
                .padding(4)
                .height(Length::Fill)
                .width(32),
            button(svg(velvet.icons.import.clone()))
                .on_press(Message::OpenImportDialog)
                .style(theme::button_style)
                .padding(4)
                .height(Length::Fill)
                .width(32),
            button(svg(velvet.icons.export.clone()))
                .on_press(Message::OpenExportDialog)
                .style(theme::button_style)
                .padding(4)
                .height(Length::Fill)
                .width(32)
        ]
        .spacing(5)
        .height(32)
        .width(320),
        space().height(10),
        button(button_message)
            .on_press(Message::Install)
            .style(theme::button_style),
    ]
    .spacing(5)
    .padding(10)
    .align_x(Alignment::Center)
    .width(Length::Fill)
    .height(Length::Fill);

    if let Some(message) = extra_message {
        main_column = main_column.push(space().height(Length::Fill));
        main_column = main_column.push(message);
    }

    main_column
}
