# Icons

This file documents the icons available in the `/icons` directory and provides guidelines for icon creation.

## Templates

- `IconMaster.svg`: A template with safe zones for all major platforms, as well as combined safe zones (default view).
- `repository-open-graph-template.png`: Default Open Graph image (a.k.a. GitHub Repo Card Template) for repository previews.

## Project Icons

> [!NOTE] Add, replace, or update icons as needed for your project.
> Update this file to describe any new icons you add.

## Universal Requirements

### Size Requirements

If you are looking for pixel perfect scaling, the least common multiple of these sizes is 3072x3072. When designing the vector source file (SSVG), 1024x1024 pixels is sufficient to allow detailed icons and still scale to most sizes. The lowest three, 16x16, 24x24, and 32x32 usually require manual tweaking no matter how you generate the master.

- [ ] Generate icons in the following sizes (px):
   	+ [ ] 1024×1024
   	+ [ ] 512×512
   	+ [ ] 256×256
   	+ [ ] 192×192
   	+ [ ] 64×64
   	+ [ ] 48×48
   	+ [ ] 32×32
   	+ [ ] 24×24
   	+ [ ] 16×16

### Technical Requirements

- [ ] Vector source files (SVG) available
- [ ] Pixel-perfect at small (48x48 and below) sizes
- [ ] Consistent visual weight across sizes
- [ ] Clear at all sizes
- [ ] 1px minimum detail size
- [ ] Proper antialiasing

## Apple Platform Guidelines

### Universal Requirements

- [ ] Color space: sRGB
- [ ] Gray gamma: 2.2
- [ ] Single layer artwork
- [ ] No alpha channels
- [ ] High contrast and legibility
- [ ] Consistent with Apple's design language

### iOS Requirements

- [ ] Square aspect ratio
- [ ] Safe zone: inner 90% of icon
- [ ] No transparency
- [ ] Sizes required:
   	+ [ ] 180×180 px (iPhone 6+ @3x)
   	+ [ ] 167×167 px (iPad Pro)
   	+ [ ] 152×152 px (iPad)
   	+ [ ] 120×120 px (iPhone @2x)
   	+ [ ] 87×87 px (Apple Watch)
   	+ [ ] 80×80 px (Spotlight @2x)
   	+ [ ] 76×76 px (iPad @1x)
   	+ [ ] 58×58 px (Settings @2x)

### macOS Requirements

- [ ] Square aspect ratio
- [ ] Follow Big Sur icon guidelines
- [ ] Support dark mode
- [ ] Include 3D perspective
- [ ] Sizes required:
   	+ [ ] 1024×1024 px (App Store)
   	+ [ ] 512×512 px (Marketing)
   	+ [ ] 256×256 px (Finder)
   	+ [ ] 128×128 px
   	+ [ ] 32×32 px
   	+ [ ] 16×16 px

### watchOS Requirements

- [ ] Circular design consideration
- [ ] High contrast for ambient mode
- [ ] Sizes required:
   	+ [ ] 1024×1024 px (Marketing)
   	+ [ ] 216×216 px (Notification Center)
   	+ [ ] 196×196 px (Home Screen)
   	+ [ ] 172×172 px (Short Look)
   	+ [ ] 100×100 px (Settings)
   	+ [ ] 92×92 px (App Switcher)
   	+ [ ] 88×88 px (Dock)
   	+ [ ] 87×87 px (Home Screen)
   	+ [ ] 80×80 px (Notifications)
   	+ [ ] 58×58 px (Settings)
   	+ [ ] 55×55 px (Notifications)
   	+ [ ] 48×48 px (Notifications)

### visionOS Requirements

- [ ] 3D depth consideration
- [ ] Support for spatial interface
- [ ] Sizes required:
   	+ [ ] 1024×1024 px (Marketing)

### tvOS Requirements

- [ ] Support parallax effect
- [ ] Layered artwork
- [ ] Sizes required:
   	+ [ ] 1280×768 px (Large)
   	+ [ ] 800×480 px (Medium)
   	+ [ ] 400×240 px (Small)

## Microsoft Guidelines

### Windows Requirements

- [ ] Follows Fluent Design System
- [ ] Support light/dark themes
- [ ] Clear padding around icon
- [ ] Sizes required:
   	+ [ ] 512×512 px (Store)
   	+ [ ] 192×192 px (Store)
   	+ [ ] 152×152 px (Store)
   	+ [ ] 64×64 px (Desktop)
   	+ [ ] 48×48 px (Desktop)
   	+ [ ] 32×32 px (Desktop)
   	+ [ ] 16×16 px (Desktop)

### Technical Specifications

- [ ] Format: PNG for bitmap, SVG for vector
- [ ] Color space: sRGB
- [ ] Bit depth: 32-bit (24-bit color + 8-bit alpha)
- [ ] Transparency supported
- [ ] Sharp edges on geometric shapes

## Google Guidelines

### Android Requirements

- [ ] Material Design guidelines
- [ ] Adaptive icon support
- [ ] Sizes required:
   	+ [ ] 512×512 px (Play Store)
   	+ [ ] 192×192 px (Launcher)
   	+ [ ] 144×144 px (Launcher)
   	+ [ ] 96×96 px (Launcher)
   	+ [ ] 72×72 px (Launcher)
   	+ [ ] 48×48 px (Launcher)
   	+ [ ] 36×36 px (Launcher)

### Chrome Requirements

- [ ] Support maskable icons
- [ ] Progressive Web App ready
- [ ] Sizes required:
   	+ [ ] 512×512 px
   	+ [ ] 192×192 px
   	+ [ ] 144×144 px
   	+ [ ] 96×96 px
   	+ [ ] 48×48 px

## Best Practices

### Design

- [ ] Consistent brand identity
- [ ] Recognizable at all sizes
- [ ] Simple and memorable
- [ ] Balanced visual weight
- [ ] Appropriate negative space
- [ ] Color contrast meets WCAG 2.2 AAA

### Technical

- [ ] Optimize file sizes
- [ ] Clean vector paths
- [ ] Proper metadata
- [ ] Version control for source files
- [ ] Documentation of color values
- [ ] Export automation setup

### Testing

- [ ] Verify all sizes
- [ ] Test on all platforms
- [ ] Check dark/light modes
- [ ] Validate accessibility
- [ ] Review in context
- [ ] Compare with competitors
