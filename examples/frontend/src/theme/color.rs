use dominator::class;
use std::sync::LazyLock;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Color {
    TextPrimary,
    TextBody,
    TextSecondary,
    TextTertiary,
    TextBrand,
    TextDisabled,
    TextInteractiveActive,
    TextInteractiveWarning,
    TextInteractiveError,
    TextInteractiveValid,
    TextInteractiveButton,
    TextInteractiveBrandDisabled,

    BorderBase,
    BorderPrimary,
    BorderSecondary,
    BorderInteractiveHover,
    BorderInteractiveSelected,
    BorderInteractiveFocus,
    BorderInteractiveDisabled,
    BorderInteractiveActive,
    BorderInteractiveError,
    BorderInteractiveValid,

    Background,
    BackgroundBrand,
    BackgroundPrimary,
    BackgroundSecondary,
    BackgroundTertiary,
    BackgroundButton,
    BackgroundInteractiveHover,
    BackgroundInteractiveSelected,
    BackgroundInteractivePressed,
    BackgroundInteractiveDisabled,
}

impl Color {
    pub const fn hex_str(self) -> &'static str {
        match self {
            // Text Colors
            Self::TextPrimary => "rgba(255, 255, 255, 1)",
            Self::TextBody => "rgba(243, 246, 248, 0.95)",
            Self::TextSecondary => "rgba(243, 246, 248, 0.7)",
            Self::TextTertiary => "rgba(243, 246, 248, 0.5)",
            Self::TextBrand => "rgba(123, 97, 255, 0.95)",
            Self::TextDisabled => "rgba(243, 246, 248, 0.2)",
            Self::TextInteractiveActive => "rgba(179, 160, 255, 0.95)",
            Self::TextInteractiveWarning => "rgba(215, 147, 73, 0.95)",
            Self::TextInteractiveError => "rgba(199, 62, 89, 0.95)",
            Self::TextInteractiveValid => "rgba(57, 166, 153, 0.95)",
            Self::TextInteractiveButton => "rgba(21, 22, 23, 0.95)",
            Self::TextInteractiveBrandDisabled => "rgba(57, 166, 153, 0.95)",

            // Border Colors
            Self::BorderBase => "rgba(0, 0, 0, 1)",
            Self::BorderPrimary => "rgba(243, 246, 248, 0.15)",
            Self::BorderSecondary => "rgba(243, 246, 248, 0.05)",
            Self::BorderInteractiveHover => "rgba(243, 246, 248, 0.15)",
            Self::BorderInteractiveSelected => "rgba(243, 246, 248, 0.2)",
            Self::BorderInteractiveFocus => "rgba(243, 246, 248, 0.2)",
            Self::BorderInteractiveDisabled => "rgba(243, 246, 248, 0.05)",
            Self::BorderInteractiveActive => "rgba(123, 97, 255, 0.65)",
            Self::BorderInteractiveError => "rgba(199, 62, 89, 0.65)",
            Self::BorderInteractiveValid => "rgba(123, 97, 255, 0.25)",

            // Background Colors
            Self::Background => "rgba(21, 22, 23, 1)",
            Self::BackgroundBrand => "rgba(123, 97, 255, 0.9)",
            Self::BackgroundPrimary => "rgba(243, 246, 248, 0.08)",
            Self::BackgroundSecondary => "rgba(243, 246, 248, 0.05)",
            Self::BackgroundTertiary => "rgba(243, 246, 248, 0.03)",
            Self::BackgroundButton => "rgba(243, 246, 248, 0.9)",
            Self::BackgroundInteractiveHover => "rgba(243, 246, 248, 0.1)",
            Self::BackgroundInteractiveSelected => "rgba(243, 246, 248, 0.15)",
            Self::BackgroundInteractivePressed => "rgba(243, 246, 248, 0.15)",
            Self::BackgroundInteractiveDisabled => "rgba(243, 246, 248, 0.03)",
        }
    }

    pub fn class(&self) -> &str {
        pub static TEXT_PRIMARY: LazyLock<String> = LazyLock::new(|| {
            class! { .style("color", Color::TextPrimary.hex_str()) }
        });
        pub static TEXT_BODY: LazyLock<String> = LazyLock::new(|| {
            class! { .style("color", Color::TextBody.hex_str()) }
        });
        pub static TEXT_SECONDARY: LazyLock<String> = LazyLock::new(|| {
            class! { .style("color", Color::TextSecondary.hex_str()) }
        });
        pub static TEXT_TERTIARY: LazyLock<String> = LazyLock::new(|| {
            class! { .style("color", Color::TextTertiary.hex_str()) }
        });
        pub static TEXT_BRAND: LazyLock<String> = LazyLock::new(|| {
            class! { .style("color", Color::TextBrand.hex_str()) }
        });
        pub static TEXT_DISABLED: LazyLock<String> = LazyLock::new(|| {
            class! { .style("color", Color::TextDisabled.hex_str()) }
        });
        pub static TEXT_INTERACTIVE_ACTIVE: LazyLock<String> = LazyLock::new(|| {
            class! { .style("color", Color::TextInteractiveActive.hex_str()) }
        });
        pub static TEXT_INTERACTIVE_WARNING: LazyLock<String> = LazyLock::new(|| {
            class! { .style("color", Color::TextInteractiveWarning.hex_str()) }
        });
        pub static TEXT_INTERACTIVE_ERROR: LazyLock<String> = LazyLock::new(|| {
            class! { .style("color", Color::TextInteractiveError.hex_str()) }
        });
        pub static TEXT_INTERACTIVE_VALID: LazyLock<String> = LazyLock::new(|| {
            class! { .style("color", Color::TextInteractiveValid.hex_str()) }
        });
        pub static TEXT_INTERACTIVE_BUTTON: LazyLock<String> = LazyLock::new(|| {
            class! { .style("color", Color::TextInteractiveButton.hex_str()) }
        });
        pub static TEXT_INTERACTIVE_BRAND_DISABLED: LazyLock<String> = LazyLock::new(|| {
            class! { .style("color", Color::TextInteractiveBrandDisabled.hex_str()) }
        });

        match self {
            Self::TextPrimary => &*TEXT_PRIMARY,
            Self::TextBody => &*TEXT_BODY,
            Self::TextSecondary => &*TEXT_SECONDARY,
            Self::TextTertiary => &*TEXT_TERTIARY,
            Self::TextBrand => &*TEXT_BRAND,
            Self::TextDisabled => &*TEXT_DISABLED,
            Self::TextInteractiveActive => &*TEXT_INTERACTIVE_ACTIVE,
            Self::TextInteractiveWarning => &*TEXT_INTERACTIVE_WARNING,
            Self::TextInteractiveError => &*TEXT_INTERACTIVE_ERROR,
            Self::TextInteractiveValid => &*TEXT_INTERACTIVE_VALID,
            Self::TextInteractiveButton => &*TEXT_INTERACTIVE_BUTTON,
            Self::TextInteractiveBrandDisabled => &*TEXT_INTERACTIVE_BRAND_DISABLED,
            _ => "",
        }
    }

    pub fn background_class(&self) -> &str {
        pub static BACKGROUND: LazyLock<String> = LazyLock::new(|| {
            class! { .style("background-color", Color::Background.hex_str()) }
        });
        pub static BACKGROUND_BRAND: LazyLock<String> = LazyLock::new(|| {
            class! { .style("background-color", Color::BackgroundBrand.hex_str()) }
        });
        pub static BACKGROUND_PRIMARY: LazyLock<String> = LazyLock::new(|| {
            class! { .style("background-color", Color::BackgroundPrimary.hex_str()) }
        });
        pub static BACKGROUND_SECONDARY: LazyLock<String> = LazyLock::new(|| {
            class! { .style("background-color", Color::BackgroundSecondary.hex_str()) }
        });
        pub static BACKGROUND_TERTIARY: LazyLock<String> = LazyLock::new(|| {
            class! { .style("background-color", Color::BackgroundTertiary.hex_str()) }
        });
        pub static BACKGROUND_BUTTON: LazyLock<String> = LazyLock::new(|| {
            class! { .style("background-color", Color::BackgroundButton.hex_str()) }
        });
        pub static BACKGROUND_INTERACTIVE_HOVER: LazyLock<String> = LazyLock::new(|| {
            class! { .style("background-color", Color::BackgroundInteractiveHover.hex_str()) }
        });
        pub static BACKGROUND_INTERACTIVE_SELECTED: LazyLock<String> = LazyLock::new(|| {
            class! { .style("background-color", Color::BackgroundInteractiveSelected.hex_str()) }
        });
        pub static BACKGROUND_INTERACTIVE_PRESSED: LazyLock<String> = LazyLock::new(|| {
            class! { .style("background-color", Color::BackgroundInteractivePressed.hex_str()) }
        });
        pub static BACKGROUND_INTERACTIVE_DISABLED: LazyLock<String> = LazyLock::new(|| {
            class! { .style("background-color", Color::BackgroundInteractiveDisabled.hex_str()) }
        });

        match self {
            Self::Background => &*BACKGROUND,
            Self::BackgroundBrand => &*BACKGROUND_BRAND,
            Self::BackgroundPrimary => &*BACKGROUND_PRIMARY,
            Self::BackgroundSecondary => &*BACKGROUND_SECONDARY,
            Self::BackgroundTertiary => &*BACKGROUND_TERTIARY,
            Self::BackgroundButton => &*BACKGROUND_BUTTON,
            Self::BackgroundInteractiveHover => &*BACKGROUND_INTERACTIVE_HOVER,
            Self::BackgroundInteractiveSelected => &*BACKGROUND_INTERACTIVE_SELECTED,
            Self::BackgroundInteractivePressed => &*BACKGROUND_INTERACTIVE_PRESSED,
            Self::BackgroundInteractiveDisabled => &*BACKGROUND_INTERACTIVE_DISABLED,
            _ => "",
        }
    }

    pub fn border_class(&self) -> &str {
        pub static BORDER_BASE: LazyLock<String> = LazyLock::new(|| {
            class! { .style("border-color", Color::BorderBase.hex_str()) }
        });
        pub static BORDER_PRIMARY: LazyLock<String> = LazyLock::new(|| {
            class! { .style("border-color", Color::BorderPrimary.hex_str()) }
        });
        pub static BORDER_SECONDARY: LazyLock<String> = LazyLock::new(|| {
            class! { .style("border-color", Color::BorderSecondary.hex_str()) }
        });
        pub static BORDER_INTERACTIVE_HOVER: LazyLock<String> = LazyLock::new(|| {
            class! { .style("border-color", Color::BorderInteractiveHover.hex_str()) }
        });
        pub static BORDER_INTERACTIVE_SELECTED: LazyLock<String> = LazyLock::new(|| {
            class! { .style("border-color", Color::BorderInteractiveSelected.hex_str()) }
        });
        pub static BORDER_INTERACTIVE_FOCUS: LazyLock<String> = LazyLock::new(|| {
            class! { .style("border-color", Color::BorderInteractiveFocus.hex_str()) }
        });
        pub static BORDER_INTERACTIVE_DISABLED: LazyLock<String> = LazyLock::new(|| {
            class! { .style("border-color", Color::BorderInteractiveDisabled.hex_str()) }
        });
        pub static BORDER_INTERACTIVE_ACTIVE: LazyLock<String> = LazyLock::new(|| {
            class! { .style("border-color", Color::BorderInteractiveActive.hex_str()) }
        });
        pub static BORDER_INTERACTIVE_ERROR: LazyLock<String> = LazyLock::new(|| {
            class! { .style("border-color", Color::BorderInteractiveError.hex_str()) }
        });
        pub static BORDER_INTERACTIVE_VALID: LazyLock<String> = LazyLock::new(|| {
            class! { .style("border-color", Color::BorderInteractiveValid.hex_str()) }
        });

        match self {
            Self::BorderBase => &*BORDER_BASE,
            Self::BorderPrimary => &*BORDER_PRIMARY,
            Self::BorderSecondary => &*BORDER_SECONDARY,
            Self::BorderInteractiveHover => &*BORDER_INTERACTIVE_HOVER,
            Self::BorderInteractiveSelected => &*BORDER_INTERACTIVE_SELECTED,
            Self::BorderInteractiveFocus => &*BORDER_INTERACTIVE_FOCUS,
            Self::BorderInteractiveDisabled => &*BORDER_INTERACTIVE_DISABLED,
            Self::BorderInteractiveActive => &*BORDER_INTERACTIVE_ACTIVE,
            Self::BorderInteractiveError => &*BORDER_INTERACTIVE_ERROR,
            Self::BorderInteractiveValid => &*BORDER_INTERACTIVE_VALID,
            _ => "",
        }
    }
}
