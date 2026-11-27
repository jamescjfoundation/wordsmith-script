# Installing and updating

Wordsmith Script is downloaded from its
[releases on GitHub](https://github.com/jamescjfoundation/wordsmith-script/releases),
and from nowhere else. A release has one file for each system and a file of
checksums:

| Your computer | The file | What it is |
| --- | --- | --- |
| A Mac with Apple silicon, macOS 11 or later | `wordsmith-script-<version>-macos-aarch64.dmg` | A disk image |
| Windows, 64-bit (x64) | `wordsmith-script-<version>-windows-x64-portable.zip` | A zip archive; nothing is installed system-wide |
| Linux, x86_64 | `wordsmith-script-<version>-linux-x86_64.tar.gz` | A compressed archive |
| | `SHA256SUMS.txt` | The checksums of the three |

There is no file for a Mac with an Intel processor, for Windows on ARM or for
Linux on ARM.

Every file holds the app (`wordsmith-script`), the command line
(`wordsmith-script-cli`, see [Command line and agents](command-line-and-agents.md))
and the license texts of the app, its fonts and every library it is built from.

The programs are not signed with a developer's certificate yet, so macOS and
Windows ask before they open the app the first time. The steps below say what
each shows and how to go on.

## macOS

1. Open the `.dmg` and drag **Wordsmith Script** onto **Applications**.
2. Open it from Applications. The first time, macOS says it cannot check the
   app and does not open it. Close that message.
3. Open **System Settings › Privacy & Security**, scroll to **Security**, and
   choose **Open Anyway** beside the line about Wordsmith Script. Confirm with
   your password or Touch ID.

On macOS 14 and earlier you can instead Control-click the app in Applications,
choose **Open**, and then **Open** again. Either way you are asked once; after
that the app opens like any other.

The image's **Legal** folder holds the license texts. They are inside the app
too.

## Windows

1. Unpack the `.zip` wherever you keep programs: your user folder is fine.
   The folder runs from where it is; nothing else is written.
2. Start `wordsmith-script.exe`. The first time, Windows shows "Windows
   protected your PC". Choose **More info**, then **Run anyway**.
3. To open `.wss` scripts with a double-click, run `register-file-types.cmd`
   in the same folder. It registers `.wss` for your account and offers
   Wordsmith Script for Fountain files (`.fountain`, `.spmd`) without taking
   them from another program. Run it again if you move the folder;
   `unregister-file-types.cmd` takes the entries away.

To remove the app, run `unregister-file-types.cmd` and delete the folder.

## Linux

```sh
tar -xzf wordsmith-script-<version>-linux-x86_64.tar.gz
cd wordsmith-script-<version>-linux-x86_64
./install.sh
```

`install.sh` copies the programs to `~/.local/bin` and the menu entry, the
icons, the `.wss` file type and the license texts to `~/.local/share`. It needs
no administrator rights. `PREFIX=<folder> ./install.sh` installs under another
folder. Start **Wordsmith Script** from the menu, or `wordsmith-script` in a
terminal when `~/.local/bin` is on your `PATH`.

You can also run `bin/wordsmith-script` from the unpacked folder without
installing.

`./uninstall.sh` (with the same `PREFIX`, if you gave one) removes what was
installed. Your scripts and settings are left alone.

The programs need glibc 2.39 or newer (Ubuntu 24.04, Debian 13, Fedora 40 and
later).

## Checking a download

Compare the file with its line in `SHA256SUMS.txt` from the same release. On
Linux, with both files in one folder:

```sh
sha256sum --check --ignore-missing SHA256SUMS.txt
```

On macOS, `shasum -a 256 --check --ignore-missing SHA256SUMS.txt`. On Windows,
`certutil -hashfile <file> SHA256` prints the checksum to compare by eye.

Each file also has a build provenance attestation: a signed statement, kept by
GitHub, that it was built from this repository by the release workflow. With
the [GitHub CLI](https://cli.github.com):

```sh
gh attestation verify wordsmith-script-<version>-linux-x86_64.tar.gz \
  --repo jamescjfoundation/wordsmith-script \
  --signer-repo jamescjfoundation/open-source-core
```

## Updating

The app looks for a newer release when it starts
([Updates and privacy](the-window.md#updates-and-privacy) says what that sends
and how to turn it off). What happens when there is one:

- **Linux:** a copy installed with `install.sh` (or kept anywhere under your
  home folder) can install the update itself and start the new version.
- **macOS and Windows:** the app opens the release page. Download the new file
  and replace the old app or folder with it. Your scripts and settings are
  kept.
