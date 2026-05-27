#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use keygen_common::{APP_TYPES, RuntimeEnvironment, env_assignment};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;
use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

const DEFAULT_LOCAL_LISTEN_ADDRESS: &str = "127.0.0.1:45632";
const SERVICE_NAME: &str = "KeyGenService";
const REQUIRED_BACKEND_MODE: &str = "local-queue-v1";
const SERVICE_ENV_OPTIONS: [&str; 12] = [
    "PGHOST",
    "PGPORT",
    "PGDATABASE",
    "PGUSER",
    "PGPASSWORD",
    "PGSSLMODE",
    "PGCONNECT_TIMEOUT",
    "KEYGEN_LISTEN_ADDRESS",
    "KEYGEN_DATA_DIR",
    "KEYGEN_PRIMARY_LOG",
    "KEYGEN_STATUS_INTERVAL_SECONDS",
    "KEYGEN_UPLOAD_INTERVAL_SECONDS",
];

type AppResult<T> = Result<T, String>;

#[derive(Clone, Debug, PartialEq)]
enum LaunchMode {
    Gui,
    Install,
    Uninstall,
    Help,
    Invalid(String),
}

#[derive(Deserialize)]
struct GenerateResponse {
    activation_key: String,
}

#[derive(Deserialize)]
struct HealthResponse {
    backend_mode: Option<String>,
}

#[derive(Serialize)]
struct GenerateRequest<'a> {
    request_code: &'a str,
    app_type: &'a str,
}

fn configured_local_generate_url() -> AppResult<String> {
    configured_local_endpoint_url("generate_key")
}

fn configured_local_health_url() -> AppResult<String> {
    configured_local_endpoint_url("health")
}

fn configured_local_endpoint_url(endpoint: &str) -> AppResult<String> {
    let listen_address = configured_value("KEYGEN_LISTEN_ADDRESS")
        .unwrap_or_else(|| DEFAULT_LOCAL_LISTEN_ADDRESS.to_owned());
    local_endpoint_url(&listen_address, endpoint)
}

fn local_endpoint_url(listen_address: &str, endpoint: &str) -> AppResult<String> {
    let address: SocketAddr = listen_address
        .trim()
        .parse()
        .map_err(|_| "KEYGEN_LISTEN_ADDRESS doit etre une adresse locale valide.".to_owned())?;
    Ok(format!("http://{address}/{endpoint}"))
}

fn configured_value(name: &str) -> Option<String> {
    RuntimeEnvironment::load()
        .ok()
        .and_then(|environment| environment.value(name))
}

fn runtime_environment() -> AppResult<RuntimeEnvironment> {
    RuntimeEnvironment::load()
        .map_err(|error| format!("Lecture du fichier .env impossible: {error}"))
}

fn validate_request_code(value: &str) -> AppResult<String> {
    let request_code = value.trim().to_ascii_uppercase();
    if request_code.len() == 14
        && request_code.as_bytes()[4] == b'-'
        && request_code.as_bytes()[9] == b'-'
    {
        Ok(request_code)
    } else {
        Err("Format du code: XXXX-XXXX-XXXX.".to_owned())
    }
}

fn local_service_ready() -> AppResult<()> {
    let response = ureq::get(&configured_local_health_url()?)
        .timeout(Duration::from_secs(2))
        .call()
        .map_err(|error| format!("Service local indisponible: {error}"))?;
    let health: HealthResponse = response
        .into_json()
        .map_err(|error| format!("Reponse du service local invalide: {error}"))?;
    match health.backend_mode.as_deref() {
        Some(REQUIRED_BACKEND_MODE) => Ok(()),
        _ => Err("Service local obsolete; mise a jour requise.".to_owned()),
    }
}

fn generate_key(request_code: &str, app_type: &str) -> AppResult<String> {
    let local_generate_url = configured_local_generate_url()?;
    let payload = GenerateRequest {
        request_code,
        app_type,
    };
    let response = ureq::post(&local_generate_url)
        .set("Content-Type", "application/json")
        .timeout(Duration::from_secs(10))
        .send_json(json!(payload))
        .map_err(|error| format!("Service local indisponible: {error}"))?;
    let response: GenerateResponse = response
        .into_json()
        .map_err(|error| format!("Reponse locale invalide: {error}"))?;
    Ok(response.activation_key)
}

fn bundle_root() -> AppResult<PathBuf> {
    if let Some(path) = env::var_os("ACTIVATEUR_BUNDLE_DIR") {
        return Ok(PathBuf::from(path));
    }
    let executable = env::current_exe()
        .map_err(|error| format!("Impossible de localiser l'application: {error}"))?;
    executable
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "Dossier de l'application introuvable.".to_owned())
}

fn install_root() -> AppResult<PathBuf> {
    env::var_os("ProgramFiles")
        .map(PathBuf::from)
        .map(|path| path.join("KeyGenRMS"))
        .ok_or_else(|| "Variable ProgramFiles introuvable.".to_owned())
}

fn run_command(program: &Path, arguments: &[&str]) -> AppResult<()> {
    let output = Command::new(program)
        .args(arguments)
        .output()
        .map_err(|error| format!("Echec de lancement de {}: {error}", program.display()))?;
    if output.status.success() {
        return Ok(());
    }
    let details = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    Err(format!(
        "Commande echouee ({}): {}",
        program.display(),
        details
    ))
}

fn run_optional(program: &Path, arguments: &[&str]) {
    let _ = Command::new(program).args(arguments).output();
}

fn launch_mode<I, S>(arguments: I) -> LaunchMode
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let arguments: Vec<String> = arguments
        .into_iter()
        .map(|argument| argument.as_ref().to_owned())
        .collect();
    match arguments.as_slice() {
        [] => LaunchMode::Gui,
        [argument] if argument == "--install" => LaunchMode::Install,
        [argument] if argument == "--uninstall" || argument == "--unstall" => LaunchMode::Uninstall,
        [argument] if argument == "--help" || argument == "-h" => LaunchMode::Help,
        _ => LaunchMode::Invalid(arguments.join(" ")),
    }
}

fn bundled_service_path() -> AppResult<PathBuf> {
    let service = bundle_root()?
        .join("KeyGenService")
        .join("KeyGenService.exe");
    if !service.is_file() {
        return Err(format!("Backend Rust manquant: {}", service.display()));
    }
    Ok(service)
}

fn bundled_nssm_path() -> AppResult<PathBuf> {
    let nssm = bundle_root()?.join("nssm").join("nssm.exe");
    if !nssm.is_file() {
        return Err(format!("NSSM manquant: {}", nssm.display()));
    }
    Ok(nssm)
}

fn verify_install_authorization(service: &Path) -> AppResult<()> {
    let environment = runtime_environment()?;
    let mut command = Command::new(service);
    command.arg("--authorize-install");
    if environment.exists() {
        command.env("KEYGEN_ENV_FILE", environment.path());
    }
    let output = command
        .output()
        .map_err(|error| format!("Verification PostgreSQL impossible: {error}"))?;
    if output.status.success() {
        return Ok(());
    }
    let details = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    if details.is_empty() {
        Err("Installation interdite ou connexion PostgreSQL indisponible.".to_owned())
    } else {
        Err(details)
    }
}

fn service_environment_contents() -> AppResult<String> {
    let source_environment = runtime_environment()?;
    let mut contents =
        "# Generated local service settings for PostgreSQL synchronization.\n".to_owned();
    for name in SERVICE_ENV_OPTIONS {
        if let Some(value) = source_environment.value(name) {
            contents.push_str(&env_assignment(name, &value));
        }
    }
    Ok(contents)
}

fn install_service() -> AppResult<String> {
    let source_service = bundled_service_path()?;
    let source_nssm = bundled_nssm_path()?;
    verify_install_authorization(&source_service)?;

    let destination = install_root()?;
    fs::create_dir_all(&destination)
        .map_err(|error| format!("Creation du dossier impossible: {error}"))?;
    let installed_service = destination.join("KeyGenService.exe");
    let installed_nssm = destination.join("nssm.exe");

    // Windows keeps a running executable locked; stop an existing instance before upgrading it.
    run_optional(&source_nssm, &["stop", SERVICE_NAME, "confirm"]);
    run_optional(&source_nssm, &["remove", SERVICE_NAME, "confirm"]);
    fs::copy(&source_service, &installed_service)
        .map_err(|error| format!("Copie de KeyGenService impossible: {error}"))?;
    fs::copy(&source_nssm, &installed_nssm)
        .map_err(|error| format!("Copie de NSSM impossible: {error}"))?;
    fs::write(destination.join(".env"), service_environment_contents()?)
        .map_err(|error| format!("Ecriture du fichier .env du service impossible: {error}"))?;

    let service_path = installed_service.to_string_lossy().to_string();
    let app_directory = destination.to_string_lossy().to_string();

    run_command(&installed_nssm, &["install", SERVICE_NAME, &service_path])?;
    run_command(
        &installed_nssm,
        &["set", SERVICE_NAME, "AppDirectory", &app_directory],
    )?;
    run_command(
        &installed_nssm,
        &["set", SERVICE_NAME, "Start", "SERVICE_AUTO_START"],
    )?;
    run_command(&installed_nssm, &["set", SERVICE_NAME, "AppNoConsole", "1"])?;
    run_command(&installed_nssm, &["start", SERVICE_NAME])?;

    Ok("Service Rust installe et demarre.".to_owned())
}

fn uninstall_service() -> AppResult<String> {
    let source_nssm = bundled_nssm_path()?;
    run_optional(&source_nssm, &["stop", SERVICE_NAME, "confirm"]);
    run_optional(&source_nssm, &["remove", SERVICE_NAME, "confirm"]);
    std::thread::sleep(Duration::from_millis(500));

    let destination = install_root()?;
    if destination.exists() {
        fs::remove_dir_all(&destination)
            .map_err(|error| format!("Suppression du backend local impossible: {error}"))?;
    }
    Ok("Service backend local supprime. Les donnees locales sont conservees.".to_owned())
}

#[cfg(windows)]
#[allow(unsafe_op_in_unsafe_fn)]
mod gui {
    use super::*;
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::Foundation::{
        ERROR_SERVICE_ALREADY_RUNNING, ERROR_SERVICE_DOES_NOT_EXIST, GetLastError, HWND, LPARAM,
        LRESULT, WPARAM,
    };
    use windows_sys::Win32::Graphics::Gdi::{COLOR_WINDOW, UpdateWindow};
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::System::Services::{
        CloseServiceHandle, OpenSCManagerW, OpenServiceW, QueryServiceStatus, SC_MANAGER_CONNECT,
        SERVICE_QUERY_STATUS, SERVICE_RUNNING, SERVICE_START, SERVICE_STATUS, StartServiceW,
    };
    use windows_sys::Win32::UI::Controls::EM_SETREADONLY;
    use windows_sys::Win32::UI::Shell::{IsUserAnAdmin, ShellExecuteW};
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

    const ID_APP_TYPE: i32 = 101;
    const ID_CODE: i32 = 102;
    const ID_KEY: i32 = 103;
    const ID_GENERATE: i32 = 201;

    #[derive(Clone, Copy)]
    struct Bounds {
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    }

    impl Bounds {
        const fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
            Self {
                x,
                y,
                width,
                height,
            }
        }
    }

    struct Controls {
        app_type: HWND,
        code: HWND,
        key: HWND,
        status: HWND,
    }

    #[derive(Clone, Copy)]
    enum RegisteredServiceState {
        Missing,
        Stopped,
        Running,
    }

    pub fn run() -> AppResult<()> {
        runtime_environment()?;
        unsafe {
            let instance = GetModuleHandleW(null());
            let class_name = wide("ActivateurRmsNativeWindow");
            let window_class = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(window_proc),
                hInstance: instance,
                lpszClassName: class_name.as_ptr(),
                hCursor: LoadCursorW(null_mut(), IDC_ARROW),
                hbrBackground: (COLOR_WINDOW + 1) as _,
                ..std::mem::zeroed()
            };
            if RegisterClassW(&window_class) == 0 {
                return Err("Impossible d'enregistrer la fenetre.".to_owned());
            }
            let title = wide("Activateur RMS - Rust");
            let window = CreateWindowExW(
                0,
                class_name.as_ptr(),
                title.as_ptr(),
                WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                570,
                310,
                null_mut(),
                null_mut(),
                instance,
                null(),
            );
            if window.is_null() {
                return Err("Impossible de creer la fenetre.".to_owned());
            }
            ShowWindow(window, SW_SHOW);
            UpdateWindow(window);

            let mut message: MSG = std::mem::zeroed();
            while GetMessageW(&mut message, null_mut(), 0, 0) > 0 {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        Ok(())
    }

    pub fn run_cli_command(mode: LaunchMode) -> i32 {
        unsafe {
            match mode {
                LaunchMode::Help => {
                    notify(
                        "Commandes",
                        "ActivateurRMS.exe --install\nActivateurRMS.exe --uninstall\n\n--unstall est aussi accepte comme alias.",
                        false,
                    );
                    0
                }
                LaunchMode::Invalid(arguments) => {
                    notify(
                        "Option invalide",
                        &format!(
                            "Option invalide: {arguments}\n\nUtilisez --install ou --uninstall."
                        ),
                        true,
                    );
                    2
                }
                LaunchMode::Install | LaunchMode::Uninstall => {
                    if IsUserAnAdmin() == 0 {
                        return match relaunch_cli_elevated(&mode) {
                            Ok(()) => 0,
                            Err(error) => {
                                notify("Elevation impossible", &error, true);
                                1
                            }
                        };
                    }
                    let result =
                        match mode {
                            LaunchMode::Install => install_service().and_then(|message| {
                                if wait_for_local_service() {
                                    Ok(message)
                                } else {
                                    Err("Service installe mais le backend local ne repond pas."
                                        .to_owned())
                                }
                            }),
                            LaunchMode::Uninstall => uninstall_service().and_then(|message| {
                                match registered_service_state()? {
                                    RegisteredServiceState::Missing => Ok(message),
                                    _ => Err(
                                        "Le service Windows existe encore apres la suppression."
                                            .to_owned(),
                                    ),
                                }
                            }),
                            _ => unreachable!(),
                        };
                    match result {
                        Ok(message) => {
                            notify("Activateur RMS", &message, false);
                            0
                        }
                        Err(error) => {
                            notify("Operation impossible", &error, true);
                            1
                        }
                    }
                }
                LaunchMode::Gui => 0,
            }
        }
    }

    pub fn show_fatal_error(error: &str) {
        unsafe {
            message_box(null_mut(), "Activateur RMS", error);
        }
    }

    unsafe extern "system" fn window_proc(
        window: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match message {
            WM_CREATE => {
                let controls = Box::into_raw(Box::new(create_controls(window)));
                SetWindowLongPtrW(window, GWLP_USERDATA, controls as _);
                initialize_backend(window, &*controls);
                0
            }
            WM_COMMAND => {
                let command = (wparam & 0xffff) as i32;
                let controls = control_state(window);
                if !controls.is_null() && command == ID_GENERATE {
                    generate_clicked(&*controls);
                }
                0
            }
            WM_DESTROY => {
                let controls = control_state(window);
                if !controls.is_null() {
                    drop(Box::from_raw(controls));
                    SetWindowLongPtrW(window, GWLP_USERDATA, 0);
                }
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(window, message, wparam, lparam),
        }
    }

    unsafe fn create_controls(window: HWND) -> Controls {
        label(window, "Logiciel", 24, 20, 130, 22);
        let app_type = CreateWindowExW(
            0,
            wide("COMBOBOX").as_ptr(),
            null(),
            WS_CHILD | WS_VISIBLE | WS_VSCROLL | CBS_DROPDOWNLIST as u32,
            24,
            44,
            230,
            120,
            window,
            ID_APP_TYPE as _,
            GetModuleHandleW(null()),
            null(),
        );
        for item in APP_TYPES {
            let item = wide(item);
            SendMessageW(app_type, CB_ADDSTRING, 0, item.as_ptr() as _);
        }
        SendMessageW(app_type, CB_SETCURSEL, 0, 0);
        label(window, "Identifiant", 278, 20, 250, 22);
        let code = edit(window, "", ID_CODE, Bounds::new(278, 44, 251, 27), false);
        label(window, "Cle d'activation", 24, 90, 500, 22);
        let key = edit(window, "", ID_KEY, Bounds::new(24, 114, 505, 27), false);
        SendMessageW(key, EM_SETREADONLY, 1, 0);
        button(window, "Generer la cle", ID_GENERATE, 24, 163, 505, 38);
        let status = label(window, "Preparation du service local...", 24, 219, 510, 35);
        Controls {
            app_type,
            code,
            key,
            status,
        }
    }

    unsafe fn initialize_backend(window: HWND, controls: &Controls) {
        let registered_service = match registered_service_state() {
            Ok(state) => state,
            Err(error) => {
                set_text(controls.status, &error);
                message_box(window, "Verification du service impossible", &error);
                return;
            }
        };
        let mut service_needs_repair = false;

        if matches!(registered_service, RegisteredServiceState::Stopped) {
            set_text(controls.status, "Demarrage du service local installe...");
            UpdateWindow(controls.status);
            if start_registered_service().is_err() {
                if IsUserAnAdmin() == 0 {
                    set_text(
                        controls.status,
                        "Autorisation Windows requise pour demarrer le service...",
                    );
                    if let Err(error) = relaunch_elevated(window) {
                        set_text(controls.status, &error);
                    }
                    return;
                }
                service_needs_repair = true;
                set_text(
                    controls.status,
                    "Service local endommage; verification avant reparation...",
                );
                UpdateWindow(controls.status);
            }
        }

        if !service_needs_repair
            && !matches!(registered_service, RegisteredServiceState::Missing)
            && wait_for_local_service()
        {
            set_text(
                controls.status,
                "Pret. Saisissez l'identifiant puis generez la cle.",
            );
            return;
        }

        set_text(
            controls.status,
            "Connexion requise: verification PostgreSQL avant installation...",
        );
        UpdateWindow(controls.status);
        let source_service = match bundled_service_path() {
            Ok(path) => path,
            Err(error) => {
                set_text(controls.status, &error);
                message_box(window, "Installation impossible", &error);
                return;
            }
        };
        if let Err(error) = verify_install_authorization(&source_service) {
            set_text(controls.status, &error);
            message_box(window, "Installation impossible", &error);
            return;
        }

        if IsUserAnAdmin() == 0 {
            set_text(
                controls.status,
                "Autorisation Windows requise pour preparer le service...",
            );
            UpdateWindow(controls.status);
            if let Err(error) = relaunch_elevated(window) {
                set_text(controls.status, &error);
            }
            return;
        }

        set_text(
            controls.status,
            "Installation automatique du service local...",
        );
        UpdateWindow(controls.status);
        match install_service() {
            Ok(_) if wait_for_local_service() => set_text(
                controls.status,
                "Pret. Saisissez l'identifiant puis generez la cle.",
            ),
            Ok(_) => {
                let error = "Service installe mais indisponible; verifiez la connexion et l'etat PostgreSQL.";
                set_text(controls.status, error);
                message_box(window, "Service indisponible", error);
            }
            Err(error) => {
                set_text(controls.status, &error);
                message_box(window, "Installation impossible", &error);
            }
        }
    }

    unsafe fn registered_service_state() -> AppResult<RegisteredServiceState> {
        let manager = OpenSCManagerW(null(), null(), SC_MANAGER_CONNECT);
        if manager.is_null() {
            return Err(format!(
                "Acces au gestionnaire de services impossible ({}).",
                GetLastError()
            ));
        }
        let service_name = wide(SERVICE_NAME);
        let service = OpenServiceW(manager, service_name.as_ptr(), SERVICE_QUERY_STATUS);
        if service.is_null() {
            let error = GetLastError();
            CloseServiceHandle(manager);
            return if error == ERROR_SERVICE_DOES_NOT_EXIST {
                Ok(RegisteredServiceState::Missing)
            } else {
                Err(format!("Lecture du service impossible ({error})."))
            };
        }
        CloseServiceHandle(manager);

        let mut status: SERVICE_STATUS = std::mem::zeroed();
        let queried = QueryServiceStatus(service, &mut status);
        let error = GetLastError();
        CloseServiceHandle(service);
        if queried == 0 {
            return Err(format!("Etat du service inaccessible ({error})."));
        }
        if status.dwCurrentState == SERVICE_RUNNING {
            Ok(RegisteredServiceState::Running)
        } else {
            Ok(RegisteredServiceState::Stopped)
        }
    }

    unsafe fn start_registered_service() -> AppResult<()> {
        let manager = OpenSCManagerW(null(), null(), SC_MANAGER_CONNECT);
        if manager.is_null() {
            return Err(format!(
                "Acces au gestionnaire de services impossible ({}).",
                GetLastError()
            ));
        }
        let service_name = wide(SERVICE_NAME);
        let service = OpenServiceW(manager, service_name.as_ptr(), SERVICE_START);
        CloseServiceHandle(manager);
        if service.is_null() {
            return Err(format!(
                "Demarrage du service non autorise ({}).",
                GetLastError()
            ));
        }
        let started = StartServiceW(service, 0, null());
        let error = GetLastError();
        CloseServiceHandle(service);
        if started == 0 && error != ERROR_SERVICE_ALREADY_RUNNING {
            Err(format!("Demarrage du service impossible ({error})."))
        } else {
            Ok(())
        }
    }

    fn wait_for_local_service() -> bool {
        for _ in 0..8 {
            if local_service_ready().is_ok() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        false
    }

    unsafe fn generate_clicked(controls: &Controls) {
        let request_code = match validate_request_code(&get_text(controls.code)) {
            Ok(code) => code,
            Err(error) => {
                set_text(controls.status, &error);
                return;
            }
        };
        let selection = SendMessageW(controls.app_type, CB_GETCURSEL, 0, 0) as usize;
        let app_type = APP_TYPES.get(selection).copied().unwrap_or("Restaurant");
        set_text(controls.status, "Generation en cours...");
        UpdateWindow(controls.status);
        match generate_key(&request_code, app_type) {
            Ok(key) => {
                set_text(controls.key, &key);
                set_text(
                    controls.status,
                    "Cle generee et enregistree; synchronisation automatique active.",
                );
            }
            Err(error) => set_text(controls.status, &error),
        }
    }

    unsafe fn relaunch_elevated(window: HWND) -> AppResult<()> {
        let executable = match env::current_exe() {
            Ok(path) => path,
            Err(error) => {
                return Err(format!("Relance impossible: {error}"));
            }
        };
        let verb = wide("runas");
        let executable = wide(&executable.to_string_lossy());
        let outcome = ShellExecuteW(
            window,
            verb.as_ptr(),
            executable.as_ptr(),
            null(),
            null(),
            SW_SHOWNORMAL,
        ) as isize;
        if outcome > 32 {
            DestroyWindow(window);
            Ok(())
        } else {
            Err("Elevation refusee ou impossible.".to_owned())
        }
    }

    unsafe fn relaunch_cli_elevated(mode: &LaunchMode) -> AppResult<()> {
        let executable =
            env::current_exe().map_err(|error| format!("Relance impossible: {error}"))?;
        let argument = match mode {
            LaunchMode::Install => "--install",
            LaunchMode::Uninstall => "--uninstall",
            _ => return Err("Commande administrative invalide.".to_owned()),
        };
        let verb = wide("runas");
        let executable = wide(&executable.to_string_lossy());
        let arguments = wide(argument);
        let outcome = ShellExecuteW(
            null_mut(),
            verb.as_ptr(),
            executable.as_ptr(),
            arguments.as_ptr(),
            null(),
            SW_SHOWNORMAL,
        ) as isize;
        if outcome > 32 {
            Ok(())
        } else {
            Err("Elevation refusee ou impossible.".to_owned())
        }
    }

    unsafe fn control_state(window: HWND) -> *mut Controls {
        GetWindowLongPtrW(window, GWLP_USERDATA) as *mut Controls
    }

    unsafe fn label(window: HWND, text: &str, x: i32, y: i32, width: i32, height: i32) -> HWND {
        CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE,
            x,
            y,
            width,
            height,
            window,
            null_mut(),
            GetModuleHandleW(null()),
            null(),
        )
    }

    unsafe fn edit(window: HWND, text: &str, id: i32, bounds: Bounds, password: bool) -> HWND {
        let mut style = WS_CHILD | WS_VISIBLE | WS_TABSTOP | ES_AUTOHSCROLL as u32;
        if password {
            style |= ES_PASSWORD as u32;
        }
        CreateWindowExW(
            WS_EX_CLIENTEDGE,
            wide("EDIT").as_ptr(),
            wide(text).as_ptr(),
            style,
            bounds.x,
            bounds.y,
            bounds.width,
            bounds.height,
            window,
            id as _,
            GetModuleHandleW(null()),
            null(),
        )
    }

    unsafe fn button(
        window: HWND,
        text: &str,
        id: i32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> HWND {
        CreateWindowExW(
            0,
            wide("BUTTON").as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON as u32,
            x,
            y,
            width,
            height,
            window,
            id as _,
            GetModuleHandleW(null()),
            null(),
        )
    }

    unsafe fn get_text(control: HWND) -> String {
        let length = GetWindowTextLengthW(control);
        let mut buffer = vec![0; (length + 1) as usize];
        GetWindowTextW(control, buffer.as_mut_ptr(), length + 1);
        String::from_utf16_lossy(&buffer[..length as usize])
    }

    unsafe fn set_text(control: HWND, value: &str) {
        SetWindowTextW(control, wide(value).as_ptr());
    }

    unsafe fn message_box(window: HWND, title: &str, value: &str) {
        MessageBoxW(
            window,
            wide(value).as_ptr(),
            wide(title).as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }

    unsafe fn notify(title: &str, value: &str, error: bool) {
        let icon = if error {
            MB_ICONERROR
        } else {
            MB_ICONINFORMATION
        };
        MessageBoxW(
            null_mut(),
            wide(value).as_ptr(),
            wide(title).as_ptr(),
            MB_OK | icon,
        );
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }
}

#[cfg(windows)]
fn main() {
    let mode = launch_mode(env::args().skip(1));
    match mode {
        LaunchMode::Gui => {
            if let Err(error) = gui::run() {
                gui::show_fatal_error(&error);
            }
        }
        mode => std::process::exit(gui::run_cli_command(mode)),
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("Activateur RMS desktop application is available on Windows only.");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_request_codes() {
        assert_eq!(
            validate_request_code("f81a-67a7-c6aa"),
            Ok("F81A-67A7-C6AA".to_owned())
        );
        assert!(validate_request_code("f81a67a7c6aa").is_err());
    }

    #[test]
    fn builds_local_endpoint_urls_from_service_listener() {
        assert_eq!(
            local_endpoint_url("127.0.0.1:45639", "generate_key"),
            Ok("http://127.0.0.1:45639/generate_key".to_owned())
        );
        assert_eq!(
            local_endpoint_url("127.0.0.1:45639", "health"),
            Ok("http://127.0.0.1:45639/health".to_owned())
        );
        assert!(local_endpoint_url("not-a-socket", "health").is_err());
    }

    #[test]
    fn parses_backend_service_command_options() {
        assert_eq!(launch_mode(Vec::<String>::new()), LaunchMode::Gui);
        assert_eq!(launch_mode(["--install"]), LaunchMode::Install);
        assert_eq!(launch_mode(["--uninstall"]), LaunchMode::Uninstall);
        assert_eq!(launch_mode(["--unstall"]), LaunchMode::Uninstall);
        assert_eq!(launch_mode(["--help"]), LaunchMode::Help);
        assert_eq!(
            launch_mode(["--other"]),
            LaunchMode::Invalid("--other".to_owned())
        );
    }
}
