# Theming Guide

Quetty features a comprehensive theming system that allows you to customize the appearance and create your own themes. This guide covers everything from using built-in themes to creating custom themes from scratch.

## Theme System Overview

Quetty's theming system is built around:
- **Theme Families**: Collections of related color schemes (e.g., Nightfox, Catppuccin)
- **Flavors**: Variants within a theme family (e.g., dark, light, different accents)
- **Color Definitions**: TOML files defining all colors used in the interface
- **Metadata**: Theme information including name, author, and icons

## Built-in Themes

### Nightfox Theme Family

A collection of themes inspired by the popular Nightfox Neovim theme.

#### Available Flavors
- **`nightfox`** - The original dark blue theme with balanced contrast
- **`duskfox`** - Darker variant with purple accents and deeper backgrounds
- **`dawnfox`** - Light theme with warm, dawn-inspired colors
- **`nordfox`** - Nord-inspired theme with cool, muted colors
- **`terafox`** - Green-accented theme with earth tones
- **`carbonfox`** - Carbon-inspired dark theme with high contrast

```toml
[theme]
theme_name = "nightfox"
flavor_name = "duskfox"  # or "nightfox", "dawnfox", "nordfox", "terafox", "carbonfox"
```

### Catppuccin Theme Family

Based on the popular Catppuccin color palette with its distinctive pastel colors.

#### Available Flavors
- **`mocha`** - Dark theme with warm, coffee-inspired colors
- **`macchiato`** - Medium-dark theme with balanced contrast
- **`frappe`** - Medium-light theme with softer colors
- **`latte`** - Light theme with cream and coffee tones

```toml
[theme]
theme_name = "catppuccin"
flavor_name = "mocha"  # or "macchiato", "frappe", "latte"
```

### Quetty Theme Family

Custom themes designed specifically for Quetty's interface.

#### Available Flavors
- **`dark`** - Custom dark theme optimized for terminal use
- **`light`** - Custom light theme with high readability

```toml
[theme]
theme_name = "quetty"
flavor_name = "dark"  # or "light"
```

## Using Themes

### Changing Themes Interactively

1. **Open Theme Picker**: Press `t` while using Quetty
2. **Browse Themes**: Navigate through available themes using `↑`/`↓`
3. **Live Preview**: See changes applied in real-time
4. **Select Theme**: Press `Enter` to apply the selected theme
5. **Auto-Save**: Your preference is automatically saved

### Configuration File

Set your preferred theme in `config.toml`:

```toml
[theme]
theme_name = "nightfox"    # Theme family
flavor_name = "duskfox"    # Specific variant
```

### Environment Variables

Override theme settings using environment variables:

```bash
export THEME__THEME_NAME="catppuccin"
export THEME__FLAVOR_NAME="mocha"
```

## Creating Custom Themes

### Theme Directory Structure

Themes are stored in the `themes/` directory:

```
themes/
├── nightfox/
│   ├── nightfox.toml
│   ├── duskfox.toml
│   ├── dawnfox.toml
│   └── ...
├── catppuccin/
│   ├── mocha.toml
│   ├── macchiato.toml
│   └── ...
├── your-theme/
│   ├── your-flavor.toml
│   └── ...
```

### Theme File Format

Each theme file is a TOML file with two main sections:

```toml
[metadata]
name = "Your Theme Name"
description = "A brief description of your theme"
author = "Your Name"
theme_name = "your-theme"      # Must match directory name
flavor_name = "your-flavor"    # Must match filename (without .toml)
theme_icon = "🎨"             # Optional icon for theme family
flavor_icon = "🌙"            # Optional icon for this flavor

[colors]
# Color definitions go here...
```

### Color Categories

Quetty uses a comprehensive set of color categories for different UI elements:

#### Core Text Colors
```toml
[colors]
text_primary = "#c0caf5"    # Primary text throughout the application
text_muted = "#565f89"      # Subtle text like separators and labels
```

#### Layout Colors
```toml
surface = "#24283b"         # Background for popups and surfaces
```

#### Accent Colors
```toml
primary_accent = "#7aa2f7"  # Focused borders, active states
title_accent = "#c0caf5"    # Component titles and headers
header_accent = "#9ece6a"   # Table headers and line numbers
```

#### Selection Colors
```toml
selection_bg = "#bb9af7"    # Selection background
selection_fg = "#1a1b26"    # Selection foreground text
```

#### Message Table Colors
```toml
message_sequence = "#e0af68"        # Message sequence numbers
message_id = "#f7768e"              # Message IDs
message_timestamp = "#7dcfff"       # Timestamps
message_delivery_count = "#bb9af7"  # Delivery counts

# Message state colors
message_state_ready = "#9ece6a"     # Ready states (Active, Scheduled)
message_state_deferred = "#e0af68"  # Deferred states
message_state_outcome = "#bb9af7"   # Outcome states (Completed, Abandoned)
message_state_failed = "#f7768e"    # Failed states (Dead-lettered)
```

#### Status Colors
```toml
status_success = "#9ece6a"  # Success messages and indicators
status_warning = "#e0af68"  # Warning messages
status_error = "#f7768e"    # Error messages
status_info = "#7dcfff"     # Informational messages
status_loading = "#bb9af7"  # Loading indicators
```

#### Help System Colors
```toml
shortcut_key = "#e0af68"         # Keyboard shortcut keys
shortcut_description = "#9aa5ce" # Help text descriptions
help_section_title = "#bb9af7"   # Help section headers
```

#### Popup System Colors
```toml
popup_background = "#1f2335"  # Popup window backgrounds
popup_text = "#9aa5ce"        # Popup text content
```

#### List Item Colors
```toml
namespace_list_item = "#bb9af7"  # Namespace picker items
queue_count = "#9ece6a"          # Queue count displays
```

### Complete Theme Example

Here's a complete custom theme file:

```toml
# themes/mytheme/dark.toml
[metadata]
name = "My Dark Theme"
description = "A custom dark theme with blue accents"
author = "Your Name"
theme_name = "mytheme"
flavor_name = "dark"
theme_icon = "🎨"
flavor_icon = "🌙"

[colors]
# Core text
text_primary = "#e1e2e7"
text_muted = "#6c7086"

# Layout
surface = "#1e1e2e"

# Accents
primary_accent = "#89b4fa"
title_accent = "#cdd6f4"
header_accent = "#a6e3a1"

# Selection
selection_bg = "#585b70"
selection_fg = "#cdd6f4"

# Message table
message_sequence = "#f9e2af"
message_id = "#f38ba8"
message_timestamp = "#89dceb"
message_delivery_count = "#cba6f7"
message_state_ready = "#a6e3a1"
message_state_deferred = "#f9e2af"
message_state_outcome = "#cba6f7"
message_state_failed = "#f38ba8"

# Status
status_success = "#a6e3a1"
status_warning = "#f9e2af"
status_error = "#f38ba8"
status_info = "#89dceb"
status_loading = "#cba6f7"

# Help system
shortcut_key = "#f9e2af"
shortcut_description = "#a6adc8"
help_section_title = "#cba6f7"

# Popups
popup_background = "#181825"
popup_text = "#a6adc8"

# Lists
namespace_list_item = "#cba6f7"
queue_count = "#a6e3a1"
```

### Color Guidelines

#### Choosing Colors

1. **Contrast**: Ensure sufficient contrast between text and background colors
2. **Accessibility**: Consider color-blind users - don't rely solely on color for information
3. **Hierarchy**: Use color to establish visual hierarchy (headers, content, metadata)
4. **Consistency**: Maintain consistent color usage across similar elements

#### Color Formats

Colors can be specified in several formats:
- **Hex**: `"#ff6b6b"` (preferred)
- **RGB**: `"rgb(255, 107, 107)"`
- **Named**: `"red"` (limited palette)

#### Color Palette Tools

Recommended tools for creating color palettes:
- [Coolors.co](https://coolors.co/) - Color palette generator
- [Adobe Color](https://color.adobe.com/) - Professional color tools
- [Paletton](https://paletton.com/) - Color scheme designer
- [Contrast Checker](https://webaim.org/resources/contrastchecker/) - Accessibility validation

### Testing Your Theme

1. **Create Theme Directory**:
   ```bash
   mkdir -p themes/mytheme
   ```

2. **Create Theme File**:
   ```bash
   # themes/mytheme/dark.toml
   # Add your theme definition here
   ```

3. **Test in Quetty**:
   - Launch Quetty
   - Press `t` to open theme picker
   - Your theme should appear in the list
   - Select it to see the preview

4. **Iterate and Refine**:
   - Make changes to the TOML file
   - Restart Quetty or re-select the theme to see changes
   - Test with different types of content

### Theme Development Best Practices

#### Design Principles

1. **Readability First**: Prioritize text readability over aesthetic appeal
2. **Context Awareness**: Consider how colors look in different terminal environments
3. **Semantic Colors**: Use colors that make semantic sense (red for errors, green for success)
4. **Terminal Compatibility**: Test in different terminals and color modes

#### Testing Checklist

- [ ] Text is readable in all contexts
- [ ] Selection highlighting is clearly visible
- [ ] Status colors are distinguishable
- [ ] Theme works in different terminal applications
- [ ] Colors look good with different background colors
- [ ] Theme is accessible to color-blind users

#### Documentation

When creating themes, consider documenting:
- Design inspiration and goals
- Target use cases (dark/light environments)
- Color palette rationale
- Accessibility considerations

## Theme Validation

Quetty automatically validates themes on startup:

### Validation Checks
- All required color fields are present
- Color values are in valid format
- Metadata fields are properly defined
- Theme/flavor names match file structure

### Error Messages
Common validation errors and solutions:

```
Error: Missing required color 'text_primary'
Solution: Add the missing color definition to [colors] section

Error: Invalid color format '#gggggg'
Solution: Use valid hex format like '#ff6b6b'

Error: Theme name mismatch
Solution: Ensure theme_name in metadata matches directory name
```

## Contributing Themes

### Sharing Your Themes

1. **Theme Quality**: Ensure your theme meets quality standards
2. **Documentation**: Include proper metadata and description
3. **Testing**: Test thoroughly across different use cases
4. **Licensing**: Specify appropriate license for your theme

### Submission Process

1. **Fork Repository**: Fork the Quetty repository
2. **Add Theme**: Add your theme to the `themes/` directory
3. **Test**: Verify theme works correctly
4. **Pull Request**: Submit a pull request with your theme
5. **Review**: Theme will be reviewed for quality and compatibility

### Theme Requirements

For inclusion in the main repository:
- Complete color definitions for all required fields
- Proper metadata with name, description, and author
- Good contrast ratios for accessibility
- Unique and meaningful theme/flavor names
- No copyright violations in color choices

## Advanced Theming

### Dynamic Theming

While not currently supported, future versions may include:
- Time-based theme switching
- Environment-aware themes
- Custom color overrides
- Theme plugins

### Terminal Integration

Some terminals support additional features:
- True color (24-bit) support
- Background transparency
- Font variations
- Custom cursors

Ensure your themes work well across different terminal capabilities.

### Performance Considerations

- Themes are loaded at startup - no runtime performance impact
- Color calculations are cached
- Multiple themes can coexist without conflicts

## Troubleshooting Themes

### Common Issues

#### Theme Not Appearing
- Check directory and file naming
- Verify TOML syntax
- Ensure all required fields are present

#### Colors Not Displaying Correctly
- Verify terminal color support
- Check color format (hex vs. RGB)
- Test in different terminals

#### Poor Readability
- Increase contrast between text and background
- Test in various lighting conditions
- Consider accessibility guidelines

### Debug Mode

Enable debug logging to troubleshoot theme issues:

```toml
[logging]
level = "debug"
file = "theme-debug.log"
```

This will log theme loading and validation information.

## Resources

### Color Theory
- [Color Theory Basics](https://www.adobe.com/creativecloud/design/discover/color-theory.html)
- [Accessibility Guidelines](https://www.w3.org/WAI/WCAG21/Understanding/contrast-minimum.html)
- [Terminal Color Standards](https://en.wikipedia.org/wiki/ANSI_escape_code#Colors)

### Inspiration
- [Terminal Color Schemes](https://github.com/mbadolato/iTerm2-Color-Schemes)
- [Vim Color Schemes](https://github.com/rafi/awesome-vim-colorschemes)
- [VS Code Themes](https://marketplace.visualstudio.com/search?target=VSCode&category=Themes)

### Tools
- [Themer](https://themer.dev/) - Multi-platform theme generator
- [Terminal.sexy](https://terminal.sexy/) - Terminal color scheme designer
- [Colorhexa](https://www.colorhexa.com/) - Color information tool

For more help with theming, see [TROUBLESHOOTING.md](TROUBLESHOOTING.md) or open an issue on GitHub.
