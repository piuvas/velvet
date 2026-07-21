use iced::{
    Alignment, Element, Length,
    widget::{
        Column, Image, button, column, image::Handle, row, scrollable, space, svg, text, text_input,
    },
};
use iced_palace::widget::ellipsized_text;

use crate::gui::{ExtraMod, Message, Velvet, theme};

const FALLBACK_ICON: &[u8] = include_bytes!("../../../res/fallback.svg");

pub fn view(velvet: &Velvet) -> Column<'_, Message> {
    let element: Element<Message> = match &velvet.modrinth_query_results {
        Some(Ok(results)) => {
            let mut mod_cards: Column<Message> = column![];
            for result in results {
                let handle = result.icon.clone();
                let image: Element<Message> = if let Some(handle) = handle {
                    // this mod has an image
                    Image::<Handle>::new(handle.as_ref())
                        .height(50)
                        .width(50)
                        .border_radius(4)
                        .into()
                } else {
                    // fallback svg copied from the modrinth website
                    svg(svg::Handle::from_memory(FALLBACK_ICON))
                        .height(50)
                        .width(50)
                        .into()
                };
                mod_cards = mod_cards.push(
                    row![
                        image,
                        space().width(8),
                        column![
                            ellipsized_text(result.title.clone()).width(Length::Fill),
                            ellipsized_text(result.description.clone())
                                .size(12)
                                .color(theme::SUBTLE)
                                .width(Length::Fill)
                        ],
                        space().width(8),
                        column![
                            space().height(Length::Fill),
                            button(svg(velvet.icons.plus.clone()))
                                .on_press(Message::AddExtraMod(ExtraMod {
                                    title: result.title.clone(),
                                    id: result.title.clone(),
                                }))
                                .style(theme::button_style)
                                .padding(4)
                                .height(30)
                                .width(30),
                            space().height(Length::Fill)
                        ],
                        space().width(16),
                    ]
                    .padding(4)
                    .height(58),
                )
            }
            mod_cards.into()
        }
        Some(Err(err)) => text(format!("error!\n{err:#?}")).color(theme::LOVE).into(),
        None => row![
            space().width(Length::Fill),
            text("Type something above to search! o_o")
                .color(theme::SUBTLE)
                .align_x(Alignment::Center),
            space().width(Length::Fill),
        ]
        .into(),
    };
    column![
        space().height(10),
        row![
            space().width(10),
            text_input("Search for mod...", &velvet.modrinth_query)
                .on_input(Message::UpdatedQuery)
                .style(theme::text_input_style)
                .padding(10)
                .width(380),
            space().width(10)
        ],
        space().height(10),
        scrollable(element).width(Length::Fill)
    ]
}
