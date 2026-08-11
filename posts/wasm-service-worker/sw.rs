//! Wasm service worker entrypoint
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, future_to_promise};
use web_sys::{Navigator, RegistrationOptions, ServiceWorkerGlobalScope, ServiceWorkerRegistration, ServiceWorkerState, console, js_sys};

// Called when the wasm module is instantiated
#[wasm_bindgen(start)]
fn init_worker() -> Result<(), JsValue> {
	let global = js_sys::global();

	if let Ok(true) = js_sys::Reflect::has(&global, &JsValue::from_str("ServiceWorkerGlobalScope")) {
		console::log_1(&JsValue::from_str("in service worker"));
		install_sw_handlers()?;
	} else {
		console::log_1(&JsValue::from_str("not in service worker"));
	}

	Ok(())
}

#[inline]
fn install_sw_handlers() -> Result<(), JsValue> {
	let global = js_sys::global();
	// we're in a service worker, so we can cast the global to a ServiceWorkerGlobalScope
	let global = global.unchecked_into::<ServiceWorkerGlobalScope>();

	// Force immediate activation
	let on_install = on_install(&global)?;
	let on_activate = on_activate(&global)?;
	global.set_oninstall(Some(on_install.as_ref().unchecked_ref()));
	global.set_onactivate(Some(on_activate.as_ref().unchecked_ref()));

	// register all callbacks
	let on_message = on_message(&global)?;
	let on_push = on_push(&global)?;
	let on_fetch = on_fetch(&global)?;
	global.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
	global.set_onpush(Some(on_push.as_ref().unchecked_ref()));
	global.set_onfetch(Some(on_fetch.as_ref().unchecked_ref()));

	// Ensure that the closures are not dropped before the service worker is terminated
	// This is technically a memory leak, but I'm not sure that it matters in this case
	on_install.forget();
	on_activate.forget();
	on_message.forget();
	on_push.forget();
	on_fetch.forget();
	Ok(())
}

/// <https://developer.mozilla.org/en-US/docs/Web/API/InstallEvent>
fn on_install(global: &ServiceWorkerGlobalScope) -> Result<Closure<dyn FnMut(web_sys::ExtendableEvent)>, JsValue> {
	let skip_waiting = global.skip_waiting()?;
	Ok(Closure::wrap(Box::new(move |event: web_sys::ExtendableEvent| {
		console::log_1(&JsValue::from_str("sw on_install"));
		// <https://developer.mozilla.org/en-US/docs/Web/API/ServiceWorkerGlobalScope/skipWaiting>
		event.wait_until(&skip_waiting).unwrap();
	}) as Box<dyn FnMut(_)>))
}

/// <https://developer.mozilla.org/en-US/docs/Web/API/ServiceWorkerGlobalScope/activate_event>
fn on_activate(global: &ServiceWorkerGlobalScope) -> Result<Closure<dyn FnMut(web_sys::ExtendableEvent)>, JsValue> {
	let clients = global.clients();
	Ok(Closure::wrap(Box::new(move |event: web_sys::ExtendableEvent| {
		console::log_1(&JsValue::from_str("sw on_activate"));
		install_sw_handlers().unwrap();
		// <https://developer.mozilla.org/en-US/docs/Web/API/Clients/claim>
		event.wait_until(&clients.claim()).unwrap();
		console::log_1(&JsValue::from_str("sw clients.claim()"));
	}) as Box<dyn FnMut(_)>))
}

/// Displays a message in the console when a message is received from the client
fn on_message(_global: &ServiceWorkerGlobalScope) -> Result<Closure<dyn FnMut(web_sys::ExtendableMessageEvent)>, JsValue> {
	Ok(Closure::wrap(Box::new(move |event: web_sys::ExtendableMessageEvent| {
		console::log_2(&JsValue::from_str("sw msg:"), &event.data());
	}) as Box<dyn FnMut(_)>))
}

/// <https://developer.mozilla.org/en-US/docs/Web/API/Push_API>
fn on_push(_global: &ServiceWorkerGlobalScope) -> Result<Closure<dyn FnMut(web_sys::PushEvent)>, JsValue> {
	Ok(Closure::wrap(Box::new(move |event: web_sys::PushEvent| {
		console::log_2(
			&JsValue::from_str("sw push:"),
			&JsValue::from_str(event.data().map(|d| d.text()).unwrap_or_else(|| "unknow".to_owned()).as_str()),
		);
	}) as Box<dyn FnMut(_)>))
}

/// <https://developer.mozilla.org/en-US/docs/Web/API/FetchEvent>
fn on_fetch(_global: &ServiceWorkerGlobalScope) -> Result<Closure<dyn FnMut(web_sys::FetchEvent)>, JsValue> {
	console::log_1(&JsValue::from_str("set onfetch"));
	Ok(Closure::wrap(Box::new(move |event: web_sys::FetchEvent| {
		console::log_2(&JsValue::from_str("sw fetch:"), &JsValue::from_str(event.client_id().unwrap_or_else(|| "unknow".to_owned()).as_str()));
		let req = event.request();
		event
			.respond_with(&future_to_promise(async move {
				let mut res_opts = web_sys::ResponseInit::new();
				res_opts.set_status(200);
				res_opts.set_headers(&req.headers().into());
				let res = web_sys::Response::new_with_opt_readable_stream_and_init(req.body().as_ref(), &res_opts)?;
				Ok(res.into())
			}))
			.unwrap();
	}) as Box<dyn FnMut(_)>))
}

////////////
// LOADER //
////////////

/// Retrieves the current service worker registration from the navigator
async fn get_service_reg(navigator: &Navigator) -> Result<ServiceWorkerRegistration, JsValue> {
	let fut = navigator.service_worker().ready()?;
	let res = JsFuture::from(fut).await?;
	Ok(ServiceWorkerRegistration::from(res))
}

fn get_worker_from_reg(reg: &ServiceWorkerRegistration) -> Option<web_sys::ServiceWorker> {
	reg.active().or_else(|| reg.waiting()).or_else(|| reg.installing())
}

/// Creates a JS promise that resolves after the given number of milliseconds and awaits it
async fn sleep(window: &web_sys::Window, ms: i32) -> Result<(), JsValue> {
	let promise = js_sys::Promise::new(&mut |resolve, _reject| {
		window.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms).unwrap();
	});
	JsFuture::from(promise).await?;
	Ok(())
}

/// This function is responsible for loading a service worker script from the given URL.
/// We use it to self load as a service worker but it could load something else.
#[wasm_bindgen]
pub async fn register_service_worker(
	worker_url: String,
	worker_type: String,
	worker_scope: String,
	_try_once: bool,
) -> Result<js_sys::Promise, JsValue> {
	console::log_1(&"registering service worker via wasm_bindgen".into());

	let window = web_sys::window().expect("no global `window` exists");
	let location = window.location();
	let navigator = window.navigator();
	let service_worker = navigator.service_worker();

	let location_href = location.href().expect("no href found");
	let url = web_sys::Url::new_with_base(&worker_url, &location_href)?;
	let url = url.to_string().as_string().unwrap();

	let mut opts = RegistrationOptions::new();
	opts.scope(&worker_scope);
	opts.type_(worker_type.as_str());

	console::log_2(&"registering service worker with opts".into(), &opts.clone().into());

	let registration_fut = service_worker.register_with_options(&url, &opts);
	let registration_res = JsFuture::from(registration_fut).await?;
	let registration = ServiceWorkerRegistration::from(registration_res);

	let registered_worker = get_worker_from_reg(&registration).ok_or_else(|| JsValue::from_str("Service worker registration is not valid"))?;

	console::log_2(&"registered service worker".into(), &registered_worker.clone().into());

	// Check to see if the registered worker is the same url
	if registered_worker.script_url() != url {
		console::log_1(&"registered worker is not the same url".into());

		let update_fut = registration.update()?;
		JsFuture::from(update_fut).await?;

		console::log_1(&"service worker updated".into());
	}

	// Await service worker to be ready
	let service_reg = get_service_reg(&navigator).await?;
	// attempt to get the service worker from the registration, if it's not there, try to re-get the registration and try again
	let service_worker = match get_worker_from_reg(&service_reg) {
		Some(worker) => worker,
		None => {
			console::log_1(&"no worker on registration, trying to re-get registration".into());
			let service_reg = get_service_reg(&navigator).await?;
			match get_worker_from_reg(&service_reg) {
				Some(worker) => worker,
				None => {
					console::log_1(&"no worker on registration, waiting a bit and trying again".into());
					sleep(&window, 50).await?;

					match get_worker_from_reg(&service_reg) {
						Some(worker) => worker,
						None => {
							console::log_1(&"no worker on registration, giving up".into());
							return Err(JsValue::from_str("Service worker registration is not valid"));
						}
					}
				}
			}
		}
	};

	match service_worker.state() {
		ServiceWorkerState::Redundant => {
			console::log_1(&"service worker is redundant".into());
			// reload
			location.reload()?;
		}
		ServiceWorkerState::Activated => {
			console::log_1(&"service worker is activated".into());
		}
		_ => {
			console::log_1(&"service worker controlling, but not activated. Waiting on event".into());
			// reload the page
			location.reload()?;
		}
	}

	Ok(js_sys::Promise::resolve(&JsValue::from(service_worker)))
}
