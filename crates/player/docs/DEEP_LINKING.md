# Deep Linking Setup

Deep linking allows users to share direct URLs or custom URI schemes to specific views in the WrldBldr Player application.

## Web (Automatic)

Browser history and URL navigation are handled automatically by the Dioxus Router.

### Features
- **Automatic URL tracking**: The address bar updates as users navigate
- **Browser history**: Back/forward buttons work correctly
- **Bookmarkable links**: Users can bookmark and share URLs to specific views
- **Deep links**: Direct navigation to any route works (with appropriate redirects if context is missing)

### URL Format

Web URLs follow standard HTTP patterns:

- `/` - Main menu
- `/roles` - Role selection
- `/worlds` - World selection
- `/worlds/{id}/dm` - DM view
- `/worlds/{id}/play` - Player view
- `/worlds/{id}/watch` - Spectator view

### Examples

```
https://wrldbldr.example.com/
https://wrldbldr.example.com/roles
https://wrldbldr.example.com/worlds
https://wrldbldr.example.com/worlds/abc-123/dm
https://wrldbldr.example.com/worlds/abc-123/play
https://wrldbldr.example.com/worlds/abc-123/watch
```

## Desktop

Desktop platforms use the `wrldbldr://` custom URI scheme for deep linking. The OS will launch the application and pass the URL to it, allowing direct navigation to specific game views.

### macOS

**Setup:**
1. The app bundle automatically registers the `wrldbldr://` scheme via `Info.plist`
2. Copy `assets/macos/Info.plist` to your app bundle's `Contents/` directory

**Usage:**
```bash
open "wrldbldr://worlds/abc-123/dm"
```

### Linux

**Setup:**
1. Install the `.desktop` file:
   ```bash
   cp assets/linux/wrldbldr.desktop ~/.local/share/applications/
   ```

2. Register the MIME type handler:
   ```bash
   xdg-mime default wrldbldr.desktop x-scheme-handler/wrldbldr
   ```

**Usage:**
```bash
xdg-open "wrldbldr://worlds/abc-123/play"
```

### Windows

**Setup:**
1. Run the registry file:
   ```bash
   regedit /s assets/windows/url-scheme.reg
   ```

   Or manually import it in Registry Editor (Windows + R → regedit)

2. Alternatively, the installer should handle this automatically

**Usage:**
- Click a link with `wrldbldr://` scheme
- Or run from command line:
  ```bash
  start "wrldbldr://worlds/abc-123/watch"
  ```

## URL Format

All desktop URLs use the `wrldbldr://` scheme with path segments:

- `wrldbldr://` - Main menu
- `wrldbldr://roles` - Role selection
- `wrldbldr://worlds` - World selection
- `wrldbldr://worlds/{id}/dm` - DM view
- `wrldbldr://worlds/{id}/play` - Player view
- `wrldbldr://worlds/{id}/watch` - Spectator view

### Examples

```
wrldbldr://
wrldbldr://roles
wrldbldr://worlds
wrldbldr://worlds/abc-123/dm
wrldbldr://worlds/test-world/play
wrldbldr://worlds/world-001/watch
```

## Mobile (Future)

Mobile platforms will be supported in future versions. Configuration will be similar to desktop:

### Android

Add to `AndroidManifest.xml`:

```xml
<intent-filter>
    <action android:name="android.intent.action.VIEW" />
    <category android:name="android.intent.category.DEFAULT" />
    <category android:name="android.intent.category.BROWSABLE" />
    <data android:scheme="wrldbldr" />
</intent-filter>
```

### iOS

Add to `Info.plist`:

```xml
<key>CFBundleURLTypes</key>
<array>
    <dict>
        <key>CFBundleURLSchemes</key>
        <array>
            <string>wrldbldr</string>
        </array>
    </dict>
</array>
```

## Implementation Details

### Storage Persistence

On web platforms, user preferences are saved to localStorage:

- `wrldbldr_server_url` - Last connected server
- `wrldbldr_role` - Selected player role
- `wrldbldr_last_world` - Last accessed world ID

These values are loaded on application startup to restore the user's session.

### URL Parsing

The application parses incoming URLs using the `url_handler` module:

```rust
// Example: parsing wrldbldr://worlds/abc-123/dm
if let Some(route) = url_handler::parse_url_scheme(url) {
    navigator.push(route);
}
```

Invalid URLs gracefully redirect to the main menu (404 handler).

### State Validation

If a user navigates directly to a view that requires session context (e.g., world selection), the application:

1. Loads saved preferences from localStorage (if available)
2. Redirects to the next setup step if required context is missing
3. Continues to the target view once all prerequisites are met

This ensures users always have a valid path through the application, even with deep links.

## Testing

### Web
```bash
# Test direct navigation
curl http://localhost:8080/worlds/test/dm

# Test browser history (manual test in browser)
# 1. Navigate to /worlds
# 2. Navigate to /roles
# 3. Click back button (should return to /worlds)
```

### Desktop
```bash
# macOS
open "wrldbldr://worlds/test/dm"

# Linux
xdg-open "wrldbldr://worlds/test/dm"

# Windows
start "wrldbldr://worlds/test/dm"
```
