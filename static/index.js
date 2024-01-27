const dropHandler = async (e) => {
  e.preventDefault();
  let arr = [...e.dataTransfer.items];
  upload(
    arr.filter((item) => item.kind == "file").map((item) => item.getAsFile()),
  );
};

const dragOverHandler = (e) => {
  e.preventDefault();
};

const uploadHandler = (e) => {
  upload(Array.from(e.target.files));
};

const showQr = (data) => {
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
};

const upload = async (files) => {
  for (let file of files) {
    if (file.size > maxSize) {
      createToast(`${file.name} is larger than ${prettyFileSize(maxSize)}`);
      continue;
    }
    document.getElementById("list-title").classList.remove("hidden");
    let log = document.createElement("div");
    let msg = `Uploading ${file.name}`;
    log.innerText = msg;
    document.getElementById("list").appendChild(log);
    createToast(msg);

    let res = await fetch(`/${file.name}`, {
      method: "PUT",
      body: file,
      headers: {
        "Content-Type": "application/octet-stream",
      },
    });
    if (res.status == 200) {
      let url = await res.text();
      createToast(`Uploaded ${url}`);
      
      let newLog = document.createElement("div");
      newLog.className = "space-x-2"
      
      let qr = document.createElement("span");
      qr.innerText = "qr";
      qr.className = "underline cursor-pointer";
      qr.onclick = () => {
        showQr(url);
      };
      newLog.appendChild(qr);
      
      let file = document.createElement("span");
      file.onclick = () => {
        navigator.clipboard.writeText(`${location.origin}/${url}`);
        createToast("Copied to clipboard");
      };
      file.className = "underline cursor-pointer";
      file.innerText = url;
      newLog.appendChild(file);


      log.replaceWith(newLog);
      fileCount++;
      document.getElementById("total").innerText = fileCount;
    } else {
      let img = document.createElement("img");
      img.src = `https://http.cat/${res.status}`;
      log.replaceWith(img);
    }
  }
};

const createToast = (msg) => {
  const toast = document.createElement("div");
  toast.innerText = msg;
  toast.className = "bg-bg border-2 border-fg p-1";
  document.getElementById("toast-container").appendChild(toast);
  setTimeout(() => {
    toast.remove();
  }, 2500);
};
