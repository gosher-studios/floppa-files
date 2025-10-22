const dropHandler = async (e) => {
  e.preventDefault();
  let arr = [...e.dataTransfer.items];
  upload(arr.filter((item) => item.kind == "file").map((item) => item.getAsFile()));
};

const dragOverHandler = (e) => {
  e.preventDefault();
};


const uploadHandler = (e) => {
  uploadChunked(Array.from(e.target.files));
  // upload(Array.from(e.target.files));
  e.target.value = null;
};

const qr = new QRCode(document.getElementById("qr"), {
  text: "",
  width: 320,
  height: 320,
  colorDark: "#000000",
  colorLight: "#ffffff",
  correctLevel: QRCode.CorrectLevel.M,
});

const blobWriter = new zip.BlobWriter("application/zip");
const writer = new zip.ZipWriter(blobWriter);


const showQr = (url) => {
  document.getElementById("qr-container").classList.replace("hidden", "fixed");
  qr.clear();
  qr.makeCode(url);
};

const closeQr = () => {
  document.getElementById("qr-container").classList.replace("fixed", "hidden");
};

const upload = async (files) => {
  if (files.length >= compressCount) {
    createToast("Zipping Files");
    for (let file of files) {
      await writer.add(file.name, new zip.TextReader(file))
    }
    await writer.close();
    files = [await blobWriter.getData()];
    files[0].name = "files.zip"
  }
  for (let file of files) {
    if (!allowEmpty && file.size == 0) {
      createToast("Empty files are disallowed");
      continue;
    }
    if (file.size > maxSize) {
      createToast(`${file.name} is larger than ${prettyFileSize(maxSize, 0)}`);
      continue;
    }
    createToast(`Uploading ${file.name}`);

    let progress = document.createElement("div");
    progress.className = "my-1 border-2 border-bg flex relative";

    let progressBar = document.createElement("div");
    progressBar.className = "absolute bg-bg left-0 inset-y-0";
    progress.appendChild(progressBar);

    let progressText = document.createElement("span");
    progressText.className = "px-1 flex-1 min-w-0 truncate z-10";
    progress.appendChild(progressText);

    let progressRight = document.createElement("span");
    progressRight.className = "px-1 z-10 hidden md:block";
    progress.appendChild(progressRight);

    document.getElementById("list").appendChild(progress);
    document.getElementById("list-title").classList.remove("hidden");

    new Promise((resolve, reject) => {
      let req = new XMLHttpRequest();
      req.open("PUT", `/${file.name}`);
      req.upload.addEventListener("progress", (e) => {
        let prog = (e.loaded / e.total) * 100.0;
        progressBar.style.width = `${prog}%`;
        progressText.innerText = `${Math.round(prog)}% ${file.name}`;
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



const getHistory = async () => {
  for (let [key, value] of Object.entries(localStorage)) {
    console.log(`${key}: ${value}`);
  }
}



// TODO i think best solution is subfunctionize everything post for i = 0;, because if we have localstorage we just need to do that :3
const uploadChunked = async (files) => {
  if (files.length >= compressCount) {
    createToast("Zipping Files");
    for (let file of files) {
      await writer.add(file.name, new zip.TextReader(file))
    }
    await writer.close();
    files = [await blobWriter.getData()];
    files[0].name = "files.zip"
  }

  const opfsRoot = await navigator.storage.getDirectory();
  for (let file of files) {
    createToast(`Uploading ${file.name}`);

    let progress = document.createElement("div");
    progress.className = "my-1 border-2 border-bg flex relative";

    let progressBar = document.createElement("div");
    progressBar.className = "absolute bg-bg left-0 inset-y-0";
    progress.appendChild(progressBar);

    let progressText = document.createElement("span");
    progressText.className = "px-1 flex-1 min-w-0 truncate z-10";
    progress.appendChild(progressText);

    let progressRight = document.createElement("span");
    progressRight.className = "px-1 z-10 hidden md:block";
    progress.appendChild(progressRight);

    document.getElementById("list").appendChild(progress);
    document.getElementById("list-title").classList.remove("hidden");
    //TODO if over size store in OPFS
    let j = await window.fetch("/begin/" + file.name + "/" + file.size)
      .then((res) => {
        if (!res.ok) {
          let err = document.createElement("span");
          err.innerText = `failed to upload ${file.name} due to ${res.statusText}`
          progress.replaceWith(err);
        }
        return res.json()
      }).then((j) => {
        return j;
      }).catch((error) => {
        let err = document.createElement("span");
        err.innerText = `failed to upload ${file.name} due to ${error};`
        progress.replaceWith(err);
      });

    const tf = await opfsRoot.getFileHandle(j.id, { create: true });
    const writable = await tf.createWritable();
    //TODO fix streaming file
    await writable.write(file);
    await writable.close();
    for (i = 0; i < j.count; i++) {
      localStorage.setItem(j.id, JSON.stringify({ count: j.count, idx: i, size: j.size, name: file.name, finished: false }));
      let prog = (i / j.count) * 100;
      progressBar.style.width = `${prog}%`;
      progressText.innerHTML = `${Math.round(prog)}% ${file.name}`;
      progressRight.innerText = `${prettyFileSize(i * j.size, 2)} / ${prettyFileSize(file.size, 2)}`;
      let t = file.slice(i * j.size, (i + 1) * j.size);
      new Promise((resolve, reject) => {
        let req = new XMLHttpRequest();
        req.open("PUT", `/up/${j.id}/${i}`);
        req.send(t);
      }).then((res) => {
        console.log(res);
      }).catch((e) => { });
    }
    window.fetch(`/end/${j.id}`).then((res) => { return res.text() }).then((t) => {
      let fileName = t;
      console.log(fileName);
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
      
    localStorage.setItem(j.id, JSON.stringify({finished: true, name: fileName}) )
    opfsRoot.removeEntry(j.id);

    }).catch((e) => { console.log("moreexploding") })

  }
}

const createToast = (msg) => {
  const toast = document.createElement("div");
  toast.innerText = msg;
  toast.className = "bg-bg border-2 border-fg p-1 w-full md:w-96";
  document.getElementById("toast-container").appendChild(toast);
  setTimeout(() => {
    toast.remove();
  }, 2500);
};
