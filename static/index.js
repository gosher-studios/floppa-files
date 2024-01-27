const dialogEl = document.getElementById("dialog");
const totalEl = document.getElementById("total");

function updateTotal() {
  totalEl.innerText = fileCount;
}

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
  for (let file of files) {
    let log = document.createElement("div");
    log.innerText = `Uploading ${file.name}`;
    document.getElementById("list").appendChild(log);

    createToast(`Uploading ${file.name}`);
    let res = await fetch(`/${file.name}`, {
      method: "PUT",
      body: file,
      headers: {
        "Content-Type": "application/octet-stream",
      },
    });
    if (res.status == 200) {
      let url = await res.text();

      let newLog = document.createElement("div");
      let show = document.createElement("div");
      let file = document.createElement("div");

      newLog.classList.add("cursor-pointer", "flex", "flex-row", "select-none");

      file.onclick = () => {
        navigator.clipboard.writeText(`${location.origin}/${url}`);
        createToast("Copied to clipboard");
      };
      file.innerText = url;
      newLog.appendChild(file);

      show.innerText = "generate qr";
      show.classList.add("px-6");
      show.onclick = () => {
        dialogEl.show();
        qr(url);
      };
      newLog.appendChild(show);

      log.replaceWith(newLog);
    } else {
      let img = document.createElement("img");
      img.src = `https://http.cat/${res.status}`;
      log.replaceWith(img);
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
