Name:           winetop
Version:        0.1.0
Release:        1%{?dist}
Summary:        htop for Wine prefixes

License:        MIT
URL:            https://github.com/akovari/winetop
Source0:        https://github.com/akovari/winetop/archive/refs/tags/v%{version}.tar.gz

BuildRequires:  cargo
BuildRequires:  rust
BuildRequires:  gcc

%description
Native CLI/TUI to monitor and stop Wine, Proton, Lutris, Heroic, and Bottles sessions.

%prep
%autosetup -n winetop-%{version}

%build
cargo build --release --locked -p winetop

%install
install -D -m 0755 target/release/winetop %{buildroot}%{_bindir}/winetop

%files
%license LICENSE
%doc README.md CHANGELOG.md
%{_bindir}/winetop

%changelog
* Tue Jul 21 2026 akovari <akovari@users.noreply.github.com> - 0.1.0-1
- Initial package
