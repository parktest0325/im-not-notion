## IM NOT NOTION
This is a tauri(rust + svelte) project that allows you to manage content by connecting to a static content site server such as hugo via ssh.   
After setting up the server within the app, you can freely edit the post.   


## DEVELOPMENT
### for MAC
```zsh
# 1. install rustup
# https://rustup.rs/
$ curl --proto '=https' --tlsv1.2 https://sh.rustup.rs -sSf | sh
$ . "$HOME/.cargo/env"

# 2. install node
# https://nodejs.org/ko

# 3. run app
$ npm install
$ npm run tauri dev
$ npm run tauri build
```

## USAGE
Windows can run released files.   
You can build it directly from this project.   
npm run tauri build   

#### for MAC
This app is unsigned, so the following steps are required:
```bash
xattr -d com.apple.quarantine /path/to/im-not-notion.app
```


## Feature
* Connect to hugo server and get list of posts   
Enter IP(Domain), Port, ID, and Password and save.   
Enter the hugo binary path, the path where the post or image is saved, and the blog's default address and save.   
After refreshing, write the post as you wish.   
![bandicam2024-03-1613-16-23-650-ezgif com-video-to-gif-converter](https://github.com/parktest0325/im-not-notion/assets/52898964/3d386015-b63e-4a93-a18b-2a5b35349b34)

* You can start editing by double-clicking and save with ctrl(cmd)+s   
![bandicam2024-03-1613-16-23-650-ezgif com-video-to-gif-converter (1)](https://github.com/parktest0325/im-not-notion/assets/52898964/e9896ec0-0edf-434b-81a0-8e87ee18d778)

* To edit the name of a file, select the item and press F2 or Enter.    
You can paste an image into a post and save it to the server. 
![bandicam2024-03-1701-19-30-025-ezgif com-video-to-gif-converter](https://github.com/parktest0325/im-not-notion/assets/52898964/b9854bf4-06ca-4f88-b173-4259ee312799)
