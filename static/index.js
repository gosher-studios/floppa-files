const dialogEl = document.getElementById("dialog");

async function dropHandler(e) {
  e.preventDefault();
  let arr = [...e.dataTransfer.items];
  upload(
    arr.filter((item) => item.kind == "file").map((item) => item.getAsFile())
  );
}

function dragOverHandler(e) {
  e.preventDefault();
}

function uploadHandler(e) {
  upload(Array.from(e.target.files));
}

function qr(data) {
  alert("unimplemented");
  /*fetch("FIXME/qr/" + data)
      .then((response) => response.text())
      .then((svg) => {
        const parser = new DOMParser();
        const qrdoc = parser.parseFromString(svg, "image/svg+xml");
        const qr = qrdoc.documentElement;
        qr.onclick = () => dialogEl.close();
        dialogEl.children[0].replaceWith(qr);
      })
      .catch((err) => {
        console.error("fuck you", err);
        return "static/download.svg";
      });*/
}

async function upload(files) {
  for (let i = 0; i < files.length; i++) {
    let file = files[i];

    let log = document.createElement("div");
    log.innerText = `Uploading ${file.name}`;
    document.getElementById("list").appendChild(log);

    createToast(`Uploading ${file.name}`);
    let res = await fetch(file.name, {
      method: "PUT",
      body: file,
      headers: {
        "Content-Type": "text/plain",
      },
    });
    if (res.status == 200) {
      let url = await res.text();

      let el = document.createElement("div");
      let file = document.createElement("div");
      file.onclick = () => {
        navigator.clipboard.writeText(`${location.origin}/${url}`);
        createToast("Copied to clipboard");
      };
      file.innerText = url.substring(9, 26);
      el.className = "cursor-pointer flex flex-row select-none ";
      log.replaceWith(el);
      el.appendChild(file);
      let show = document.createElement("div");
      show.innerText = "generate qr";
      show.className = "px-6";
      show.onclick = () => {
        dialogEl.show();
        qr(url);
      };
      el.appendChild(show);
    } else {
      log.innerHTML = `<img src="https://http.cat/${res.status}"/>`;
    }
  }
}

const container = document.getElementById("toast-container");

function createToast(message) {
  const toast = document.createElement("div");
  toast.innerText = message;
  toast.className = "text-white overflow-none bg-bg border-2 border-fg m-2 p-1";
  container.appendChild(toast);
  setTimeout(() => {
    toast.remove();
  }, 2500);
}
