<p align="center">
  <img src="public/project-garage-logo.png" alt="project garage" width="760" />
</p>

# Your cars. Your work. Your garage.

Project Garage is a personal project-car tracker built around the work that gets your car finished. Keep a record of the parts you need, what you've bought, what you've spent, and how your build looks along the way.

Android comes first, with a shared Rust foundation and a Tauri 2 app shell for Windows, macOS, and Linux. The interface uses React and TypeScript. Rust handles the garage rules and SQLite storage directly inside the installed app.

**The Android app runs independently.** Enter your name and start your garage. Cars, parts, expenses, receipts, and journal photos are saved on the device, and you can make changes offline. No local server, computer, account, or server address is required for your personal garage. Optional hosted sharing connects owners and staff across devices.

## Build status

The first working version includes the responsive interface, on-device Rust/SQLite storage, personal welcome, parts and expense tracking, photo journal, and optional shared-garage permissions. Seven Rust tests cover permission rules, hosted sharing, and reopening device data without any running server. This is an early development build, not a store-ready Android release. Physical-device testing remains to be done.

**[Download the Android test APK](downloads/project-garage-arm64-debug.apk)** · [Build notes](downloads/README.md)

## What belongs in your garage

- **Your own welcome:** “Opening Kevin's Project Garage.” Your name makes it personal; the owner can rename the garage.
- **Multiple project cars:** keep each car's details, required work, parts, and photo history together.
- **Parts from start to finish:** Needed → Purchased → Received → Installed.
- **Purchase records:** price, purchase date, seller, receipts, and applicable warranty.
- **Total spending:** see what you've actually spent on purchased parts, without labor billing or budget management.
- **Required jobs:** track wiring, paint, alignment, and other work alongside parts.
- **Dated build journal:** notes and photos show how the project came along.
- **Automatic completion:** a project is complete when every approved, required item is done. Optional items and unapproved suggestions do not count.
- **Standalone personal garage:** open the installed app and enter your name; everything is stored on your device.
- **Offline work:** add cars, edit parts, record purchases, and save progress photos with no internet connection.
- **Optional shared access:** connect to a hosted garage to work with an owner and staff. Shared garages require a connection for changes and offer offline viewing of their saved copy.

## Optional shared garages: owner and staff access

| Action                                      | Owner | Staff                 |
| ------------------------------------------- | ----- | --------------------- |
| Add, remove, or configure cars              | Yes   | No                    |
| Rename or configure the garage              | Yes   | No                    |
| Update approved parts and purchase records  | Yes   | Yes                   |
| Mark parts received or installed            | Yes   | Yes                   |
| Complete approved jobs                      | Yes   | Yes                   |
| Add dated journal entries and photos        | Yes   | Yes                   |
| Suggest parts or work                       | Yes   | Yes, pending approval |
| Approve suggestions or change required work | Yes   | No                    |
| Invite or remove staff                      | Yes   | No                    |

Staff invitations expire after seven days and can be used once. Account passwords are hashed with Argon2. The Rust service checks permissions for every change, and revision checks prevent one person's update from silently overwriting another's.

## Development setup

Install a current Rust toolchain and Node.js LTS. On Windows, Rust compilation also needs the Microsoft C++ Build Tools. Native platform prerequisites are described in [Tauri's documentation](https://v2.tauri.app/start/prerequisites/).

```sh
git clone https://github.com/dustindwayne/projectgarage.git
cd projectgarage
npm install
```

For the browser preview and optional shared-service development, start the Rust service in one terminal:

```sh
cargo run -p garage-server
```

Start the interface in a second terminal:

```sh
npm run dev
```

Open `http://localhost:1420`. Create an owner account to start your garage. Invite staff from the garage settings; they create accounts using the invitation code on the same service.

## Native apps

Start the desktop app. It stores your garage directly in its own SQLite database; no garage service is needed:

```sh
npm run tauri dev
```

Build a desktop package on the operating system you're targeting:

```sh
npm run tauri build
```

For Android, install Android Studio, its SDK/NDK, and the required Rust Android targets as described in [the Android prerequisites](https://v2.tauri.app/start/prerequisites/#android). Then:

```sh
npm run tauri android init
npm run tauri android dev
# or
npm run tauri android build -- --apk
```

On Windows, `./scripts/build-android.ps1` builds an ARM64 debug APK and copies it to `artifacts/project-garage-arm64-debug.apk`. It uses Android Studio's Java runtime and includes a file-copy fallback for machines that do not permit symbolic links. The **Android APK** GitHub Actions workflow also builds a downloadable test APK on pushes to `main`.

The installed Android and desktop apps call Rust directly and store their personal garage in the app's private data directory. They do not start a local HTTP server or listen on a port. If you choose to join a shared garage, use its hosted HTTPS address. Device garages and hosted garages remain separate; local records are not uploaded automatically.

## Optional hosted service configuration

| Variable         | Default                             | Purpose                                |
| ---------------- | ----------------------------------- | -------------------------------------- |
| `GARAGE_BIND`    | `127.0.0.1:3030`                    | Service listen address                 |
| `GARAGE_DB`      | `data/garage.db`                    | SQLite database location               |
| `GARAGE_ORIGINS` | Local development and Tauri origins | Comma-separated allowed client origins |

Build the interface with `npm run build` before using the Rust service to serve it directly at `http://127.0.0.1:3030`. Start the service from the repository root so it can find `dist/`.

For a shared installation, run the service behind an HTTPS reverse proxy, configure allowed origins, and back up the SQLite database. Hosting is not provisioned by cloning the repository. Photos and receipts are stored with the garage in SQLite; images are validated and storage limits are enforced. No analytics, advertising, labor invoicing, or tuning management is included.

## Checks

```sh
cargo test -p garage-core -p garage-server
npm run build
```

## Project layout

```text
crates/garage-core/      Car, part, journal, and permission rules
crates/garage-server/    Shared Rust storage library and optional hosted service
src/                    Responsive app interface
src-tauri/              Native app, direct Rust commands, private device database
public/                 Project Garage branding
```

Development takes place on **main**.
