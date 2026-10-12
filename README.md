<p align="center">
  <img src="public/project-garage-readme.svg" alt="project garage" width="760" />
</p>

# Your cars. Your work. Your garage.

Project Garage is a personal project-car tracker built around the work that gets your car finished. Keep a record of the parts you need, what you've bought, what you've spent, and how your build looks along the way.

Android comes first, with a shared Rust foundation and a Tauri 2 app shell for Windows, macOS, and Linux. The interface uses React and TypeScript. Rust handles the garage rules and SQLite storage directly inside the installed app.

**The Android app runs independently.** Enter your name and start your garage. Cars, parts, expenses, receipts, and journal photos are saved on the device, and you can make changes offline. No local server, computer, account, or server address is required for your personal garage. Optional hosted sharing connects owners and staff across devices.

## Build status

The app includes on-device Rust/SQLite storage, personal welcome, parts and expense tracking, photo journal, System/Light/Dark themes, and optional shared-garage permissions. Nineteen Rust tests cover permissions, acquisition totals, saved-data compatibility, persistence, Android discovery, AI record patches, atomic batches, and update verification. This is an early development build, not a store-ready Android release. Physical-device testing remains to be done.

**[Download the Android test APK](downloads/project-garage-arm64-debug.apk)** · [Build notes](downloads/README.md)

## What belongs in your garage

- **Your own welcome:** “Opening Kevin's Project Garage.” Your name makes it personal; the owner can rename the garage.
- **Multiple project cars:** keep each car's details, required work, parts, and photo history together.
- **Parts from start to finish:** Needed → Purchased → Received → Installed.
- **Purchase records:** price, purchase date, seller, receipts, and applicable warranty.
- **Where it started:** record where each car came from, its initial purchase price, and purchase date. Unknown prices remain distinct from a free car.
- **Total spending:** see car purchase cost and purchased parts separately, plus their combined total. Needed-part estimates and labor are excluded.
- **Comfortable appearance:** choose System, Light, or Dark in the welcome screen or settings. The choice is remembered on each device.
- **Mobile first:** larger text and touch targets, full-width phone actions, scrollable status filters, and clear spending and approval labels.
- **Required jobs:** track wiring, paint, alignment, and other work alongside parts.
- **Dated build journal:** notes and photos show how the project came along.
- **Automatic completion:** a project is complete when every approved, required item is done. Optional items and unapproved suggestions do not count.
- **Standalone personal garage:** open the installed app and enter your name; everything is stored on your device.
- **Offline work:** add cars, edit parts, record purchases, and save progress photos with no internet connection.
- **Optional shared access:** connect to a hosted garage to work with an owner and staff. Shared garages require a connection for changes and offer offline viewing of their saved copy.

## Nearby Android discovery

Installed desktop apps automatically listen for Project Garage on the same Wi-Fi network. Keep the Android app open and check **Settings → Nearby Android apps** on desktop. Android announces only an app identifier; garage records, photos, and account details are not broadcast. Discovery can be disabled in settings, and the choice is remembered.

Discovery uses UDP port 39073. Guest-network isolation or a firewall can prevent detection. On Android 17 and newer, allow the local-network permission when prompted. Discovery identifies devices; pairing, record transfer, and automatic synchronization are not implemented by this feature. Android still runs independently and does not host a local server.

## In-app updates from GitHub

Install **0.3.2** once to switch to token-free updates. Open **Settings → App updates** to check and download a newer version. Optional startup checks run at most every four hours. GitHub accounts and tokens are no longer required.

Installers and the manifest are published in the public [projectgarage-updates repository](https://github.com/dustindwayne/projectgarage-updates), on **main**. App source stays private in this repository. The app reads the public GitHub download feed over HTTPS without embedding or storing any GitHub credential. Size and SHA-256 checks remain mandatory. Android additionally checks package identity, newer version code, and the existing APK signer before opening the system installer. Windows closes, replaces its executable, and restarts. Garage data stays in the app's separate data directory. macOS and Linux installers are not yet published.

Older builds used the private source repository for updates, so install the new APK or Windows executable once to move to the public channel.

To publish the next update, increase the matching app versions in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`; build Android and Windows; then run:

```sh
node scripts/publish-update.mjs
```

Commit the installers, checksums, and `downloads/latest.json` together and push to `main`. Build Android updates with the **same signing key** as the installed app. Keep your signing keystore backed up securely; never commit it. The current Android CI workflow produces test artifacts with its runner's own debug key and does not publish them into this update channel.

## Owner and staff access

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
## Phone workflow and personalization (0.3.0)

Phone navigation keeps **Garage**, **Assistant**, **Spending**, and **Settings** within thumb reach. Open a car to access its **Build journal**; journals are no longer a separate garage-wide destination. Phone forms fill the screen, and the desktop workspace expands at 960px.

Under **Settings → Appearance**, choose a preset accent or a custom color. Theme and accent are remembered on this device. Spending defaults to all cars and can be filtered to one car. It separates initial car purchases from purchased parts and lists purchase dates, sellers, and saved receipts. Needed parts and pending suggestions do not count as money spent.

## Optional built-in AI assistant

The installed Android and desktop apps connect directly to **OpenAI (ChatGPT models)**, **Anthropic Claude**, or **xAI Grok** over HTTPS. Enable **Show built-in AI assistant** under **Settings → Use your own agent (optional)**, then select your provider, enter a model ID available to your account, and supply its API key. Ordinary chat subscriptions are not API credentials; provider API usage is billed separately. Keys remain in frontend/native request memory and are never saved in settings, backups, SQLite, or the repository. Close the app or choose **Disconnect and forget key** to clear access.

The assistant can answer garage questions and create, edit, delete, or update car and part records, approve suggestions, record purchases, and add or remove journal entries. Requested changes save automatically with **full control of records**, limited by the signed-in user's existing owner/staff permissions. It cannot manage member access, install updates, run shell commands, or change credentials. Garage text records are sent to the selected provider; photo and receipt binaries, journal authors, and member contact fields are omitted. Existing attachment files are preserved during partial edits. Text entered in notes is included, so avoid putting credentials in notes.

Changes use a bounded batch: invalid actions or a changed garage revision reject the whole batch. The activity view records the last ten requests in the current session. New cars must be created first before requesting their parts, because saved car IDs are assigned locally. AI has no background autonomous access and only acts when you press **Run request**. Browser previews do not receive API keys; this feature runs in installed apps. No Android local server is introduced.

Provider request formats follow the [OpenAI Chat Completions API](https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create), [Claude Messages API](https://platform.claude.com/docs/en/api/messages/create), and [xAI Chat Completions API](https://docs.x.ai/developers/rest-api-reference/inference/chat-completions). Provider access was not tested against live accounts in development; no provider keys were available. Rust tests cover attachment preservation, missing IDs, omitted identity/media, atomic rollback, permissions, persistence, discovery, and update verification.

The 0.3.2 branding update uses the supplied SVG wordmark and its open-hood car as the launcher icon. In-app logo colors follow your selected accent and theme. Dark mode uses black (#090909) and neutral charcoal (#181818 / #242424); accent colors do not tint its surfaces.

After building and generating the manifest, run `scripts/publish-public-updates.ps1` to publish only the fixed installer/manifest allowlist to the public update repository. It does not copy the source checkout or garage data.

## External agents without in-app AI setup

The main app defaults to Garage, Spending, and Settings. The built-in Assistant tab appears only when you enable it; disabling it clears its provider key. Under **Settings → Use your own agent (optional)**, copy a prompt for the agent you already use. The [portable skill](skills/project-garage-agent/SKILL.md) ships in the public download repository as a ZIP and plain instructions. No provider account or key is required inside Project Garage for this route.

On desktop, an agent with filesystem and command access uses the Rust `garage-agent` bridge to read and change the same local SQLite garage. The bridge validates partial patches through the app’s rules, checks the current revision, applies batches atomically, preserves receipt/photo fields, and leaves the app’s session signed in. It does not start a server or edit SQLite records directly. The published Windows bridge is downloaded and SHA-256 verified by the skill’s bootstrap script. macOS/Linux can build the same command bridge from this checkout.

Android and cloud chats use a file handoff: **Share garage context** opens Android’s share chooser for a text-only context JSON file. Ask the agent for changes, then choose **Import agent changes** and apply them in Settings. This handoff is required because a pasted prompt does not give an external agent access to Android’s app sandbox. It is not automatic Wi-Fi synchronization. Context files exclude photo/receipt binaries and member contacts. The user chooses the receiving agent. Stale changes require a fresh context. Browser previews can export context files; installed apps include native sharing. Physical-phone sharing is not yet tested.

Bridge examples: `garage-agent snapshot --output context.json`, `garage-agent apply --file changes.json --output result.json`, or `garage-agent init --name Kevin` to initialize a new local garage explicitly. Use `--db` only for a chosen existing database or an isolated test fixture. Read the skill’s action reference for the revisioned changes format.
