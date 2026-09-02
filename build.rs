//! Recursos do executável no Windows: ícone e manifesto.
//!
//! O manifesto responde por duas coisas que o app não consegue pedir sozinho a
//! tempo: a elevação (o `SendInput` precisa da mesma integridade da janela que
//! recebe as teclas) e a consciência de DPI por monitor, que tem de valer desde
//! o primeiro pixel — a chamada equivalente em `ui/window.rs` só existe para o
//! `cargo run`, que roda o exe sem instalador.
//!
//! Cross-compilando de fora do Windows não há compilador de recursos: o build
//! segue sem eles e avisa. O binário distribuído vem da CI, que roda no Windows.

/// Ícone de id 1 — é o que o Explorer mostra e o que a janela carrega com
/// `LoadIconW(instance, PCWSTR(1))`.
const ICON: &str = "assets/icon.ico";

const MANIFEST: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity type="win32" name="MacroHelldivers2" version="1.0.0.0"/>
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="requireAdministrator" uiAccess="false"/>
      </requestedPrivileges>
    </security>
  </trustInfo>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <!-- Windows 10 e 11 -->
      <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}"/>
      <!-- Windows 8.1 -->
      <supportedOS Id="{1f676c76-80e1-4239-95bb-83d0f6d0da78}"/>
    </application>
  </compatibility>
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
      <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/pm</dpiAware>
    </windowsSettings>
  </application>
</assembly>
"#;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={ICON}");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let mut resources = winresource::WindowsResource::new();
    resources.set_icon(ICON);
    resources.set_manifest(MANIFEST);
    if let Err(err) = resources.compile() {
        // Na CI isto é fatal: um exe publicado sem o manifesto perde a elevação
        // — e o `SendInput` no jogo elevado simplesmente para de funcionar, em
        // silêncio. No desenvolvimento fora do Windows (sem `rc.exe`/`llvm-rc`)
        // o build segue: o que se perde é só o ícone e o manifesto do exe local.
        if std::env::var_os("CI").is_some() {
            panic!("ícone e manifesto não embutidos: {err}");
        }
        println!("cargo:warning=ícone e manifesto não embutidos: {err}");
    }
}
