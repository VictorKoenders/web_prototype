function enable_auto_reload(interval) {
    setInterval(reload, interval)
}

async function reload() {
    let response = await fetch(document.location.pathname + '.json');
    let json = await response.json();
    root.model(json);
}

root = {};
function load(id, model) {
    root = {
        model: ko.observable()
    };
    root.model(model);
    ko.applyBindings(root, document.getElementById(id));
}