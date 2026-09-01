// Ponto de entrada do binário nativo. Por enquanto é só um placeholder: o
// bootstrap real (instância única, threads engine/hooks, janela principal e
// message loop) chega junto com a janela, na Fase 4 do plano de reescrita.
//
// O app só roda no Windows. No host de desenvolvimento (macOS/Linux) o crate
// compila e roda os testes dos módulos de lógica pura, mas não faz nada útil.

fn main() {
    println!(
        "{} v{} — placeholder do scaffold nativo",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );

    if !cfg!(windows) {
        println!("Este binário só é funcional no Windows; aqui ele serve para check/test.");
    }
}
