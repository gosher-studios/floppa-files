const dropHandler = async (e) => {
  e.preventDefault();
  let arr = [...e.dataTransfer.items];
  upload(arr.filter((item) => item.kind == "file").map((item) => item.getAsFile()));
};

const dragOverHandler = (e) => {
  e.preventDefault();
};

const uploadHandler = (e) => {
  upload(Array.from(e.target.files));
};

const qr = new QRCode(document.getElementById("qr"), {
  text: "",
  width: 320,
  height: 320,
  colorDark: "#000000",
  colorLight: "#ffffff",
  correctLevel: QRCode.CorrectLevel.M,
});

const showQr = (url) => {
  document.getElementById("qr-container").classList.replace("hidden", "fixed");
  qr.clear();
  qr.makeCode(url);
};

const closeQr = () => {
  document.getElementById("qr-container").classList.replace("fixed", "hidden");
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
    // ideally we should error handle if await fetch fails buttttt :3
    if (res.status == 200) {
      let fileName = await res.text();
      let url = `${location.origin}/${fileName}`;
      createToast(`Uploaded ${fileName}`);

      let newLog = document.createElement("div");
      newLog.className = "space-x-2";

      let qr = document.createElement("span");
      qr.innerText = "qr";
      qr.className = "underline cursor-pointer";
      qr.onclick = () => {
        showQr(url);
      };
      newLog.appendChild(qr);

      let file = document.createElement("span");
      file.onclick = () => {
        navigator.clipboard.writeText(url);
        createToast("Copied to clipboard");
      };
      file.className = "underline cursor-pointer";
      file.innerText = fileName;
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
