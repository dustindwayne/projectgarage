package com.projectgarage.mobile

import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.provider.Settings
import android.content.pm.PackageManager
import androidx.core.content.FileProvider
import app.tauri.annotation.Command
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Plugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import java.io.File

@TauriPlugin
class GarageUpdatesPlugin(private val activity: Activity): Plugin(activity) {
    @Command
    fun install(invoke: Invoke) {
        try {
            val file = File(invoke.getArgs().getString("path")).canonicalFile
            val allowed = File(activity.cacheDir, "updates").canonicalFile
            if (file.parentFile != allowed || file.name != "project-garage-update.apk" || !file.isFile) {
                invoke.reject("Invalid update file"); return
            }
            val info = activity.packageManager.getPackageArchiveInfo(file.path, if(Build.VERSION.SDK_INT>=28) PackageManager.GET_SIGNING_CERTIFICATES else PackageManager.GET_SIGNATURES)
            if (info == null || info.packageName != activity.packageName) {invoke.reject("This update is not for Project Garage");return}
            val current = activity.packageManager.getPackageInfo(activity.packageName, if(Build.VERSION.SDK_INT>=28) PackageManager.GET_SIGNING_CERTIFICATES else PackageManager.GET_SIGNATURES)
            val incomingVersion = if(Build.VERSION.SDK_INT>=28) info.longVersionCode else info.versionCode.toLong()
            val currentVersion = if(Build.VERSION.SDK_INT>=28) current.longVersionCode else current.versionCode.toLong()
            if(incomingVersion<=currentVersion){invoke.reject("The downloaded app is not a newer version");return}
            val incomingSigners = if(Build.VERSION.SDK_INT>=28) info.signingInfo?.apkContentsSigners else info.signatures
            val currentSigners = if(Build.VERSION.SDK_INT>=28) current.signingInfo?.apkContentsSigners else current.signatures
            if(incomingSigners==null||currentSigners==null||incomingSigners.toSet()!=currentSigners.toSet()){invoke.reject("The update signing key does not match your installed app. Nothing was installed.");return}
            if(Build.VERSION.SDK_INT>=26&&!activity.packageManager.canRequestPackageInstalls()){
                activity.startActivity(Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES,Uri.parse("package:"+activity.packageName)))
                invoke.resolve(JSObject().put("status","permission_required"));return
            }
            val uri=FileProvider.getUriForFile(activity,activity.packageName+".fileprovider",file)
            val intent=Intent(Intent.ACTION_VIEW).setDataAndType(uri,"application/vnd.android.package-archive").addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            activity.startActivity(intent)
            invoke.resolve(JSObject().put("status","installer_opened"))
        } catch(error:Exception){invoke.reject("Could not open the Android installer: "+error.message)}
    }
    @Command
    fun shareContext(invoke:Invoke) {
        try {
            val file=File(invoke.getArgs().getString("path")).canonicalFile
            val allowed=File(activity.cacheDir,"project-garage-agent-context.json").canonicalFile
            if(file!=allowed||!file.isFile){invoke.reject("Invalid agent context file");return}
            val uri=FileProvider.getUriForFile(activity,activity.packageName+".fileprovider",file)
            val send=Intent(Intent.ACTION_SEND).setType("application/json").putExtra(Intent.EXTRA_STREAM,uri).addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            activity.startActivity(Intent.createChooser(send,"Share garage context with your agent"))
            invoke.resolve(JSObject().put("status","shared"))
        } catch(error:Exception){invoke.reject("Could not share garage context: "+error.message)}
    }
}
