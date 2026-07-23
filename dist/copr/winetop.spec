Name:           winetop
Version:        0.2.0
Release:        1%{?dist}
Summary:        htop for Wine prefixes

License:        MIT
URL:            https://github.com/akovari/winetop
Source0:        https://github.com/akovari/winetop/archive/refs/tags/v%{version}.tar.gz

BuildRequires:  cargo
BuildRequires:  rust
BuildRequires:  gcc
BuildRequires:  openssl-devel
BuildRequires:  pkgconfig

%description
Native CLI/TUI to monitor and stop Wine, Proton, Lutris, Heroic, and Bottles sessions.

%prep
%autosetup -n winetop-%{version}

%build
cargo build --release --locked -p winetop

%install
install -D -m 0755 target/release/winetop %{buildroot}%{_bindir}/winetop
install -D -m 0644 man/winetop.1 %{buildroot}%{_mandir}/man1/winetop.1

%files
%license LICENSE
%doc README.md CHANGELOG.md
%{_bindir}/winetop
%{_mandir}/man1/winetop.1*

%changelog
* Thu Jul 23 2026 Adam Kovari <adam@kovari.eu> - 0.2.0-1
- Add winetop status for Waybar / status bars

* Wed Jul 22 2026 Adam Kovari <adam@kovari.eu> - 0.1.6-1
- Prune vendored static libs for Debian/Launchpad source uploads

* Wed Jul 22 2026 Adam Kovari <adam@kovari.eu> - 0.1.5-1
- Preserve vendored Cargo.toml.orig for Debian builds

* Wed Jul 22 2026 Adam Kovari <adam@kovari.eu> - 0.1.4-1
- Pin deps for Ubuntu Noble rustc 1.75 (Launchpad)

* Wed Jul 22 2026 Adam Kovari <adam@kovari.eu> - 0.1.3-1
- Fix invalid Maintainer tag that broke Copr SRPM generation

* Wed Jul 22 2026 Adam Kovari <adam@kovari.eu> - 0.1.2-1
- Fix Copr SCM build invocation

* Tue Jul 21 2026 Adam Kovari <adam@kovari.eu> - 0.1.0-1
- Initial package
