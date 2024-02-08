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
      createToast(`${file.name} is larger than ${prettyFileSize(maxSize, 0)}`);
      continue;
    }
    createToast(`Uploading ${file.name}`);

    let progress = document.createElement("div");
    progress.className = "my-1 border-2 border-bg flex justify-between";

    let progressInner = document.createElement("div");
    progressInner.className = "bg-bg pl-1 overflow-visible whitespace-nowrap";
    progress.appendChild(progressInner);

    let progressRight = document.createElement("span");
    progressRight.className = "pr-1 hidden md:block";
    progress.appendChild(progressRight);

    document.getElementById("list").appendChild(progress);
    document.getElementById("list-title").classList.remove("hidden");

    new Promise((resolve, reject) => {
      let req = new XMLHttpRequest();
      req.open("PUT", `/${file.name}`);
      req.upload.addEventListener("progress", (e) => {
        let prog = (e.loaded / file.size) * 100.0;
        progressInner.innerText = `${Math.round(prog)}% ${file.name}`;
        progressInner.style.width = `${prog}%`;
        progressRight.innerText = `${prettyFileSize(e.loaded, 2)}/${prettyFileSize(file.size, 2)}`;
      });
      req.addEventListener("error", () => reject(req));
      req.addEventListener("abort", () => reject(req));
      req.addEventListener("load", () => (req.status === 200 ? resolve(req) : reject(req)));
      req.send(file);
    })
      .then((res) => {
        let fileName = res.responseText;
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
        file.innerText = fileName;
        file.className = "underline cursor-pointer";
        file.onclick = () => {
          navigator.clipboard.writeText(url);
          createToast("Copied to clipboard");
        };
        newLog.appendChild(file);

        progress.replaceWith(newLog);
        fileCount++;
        document.getElementById("total").innerText = fileCount;
      })
      .catch(() => {
        let err = document.createElement("span");
        err.innerText = `Failed to upload ${file.name}`;
        progress.replaceWith(err);
      });
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
