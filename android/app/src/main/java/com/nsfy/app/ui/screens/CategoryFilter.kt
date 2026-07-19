package com.nsfy.app.ui.screens

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier

@Composable
fun CategoryFilter(
    paths: List<String>,
    selected: String,
    onSelected: (String) -> Unit,
    modifier: Modifier = Modifier,
) {
    var expanded by remember { mutableStateOf(false) }
    Box(modifier) {
        OutlinedButton(
            onClick = { expanded = true },
            modifier = Modifier.fillMaxWidth(),
        ) {
            Text(if (selected.isEmpty()) "全部分类" else selected.replace("/", " › "))
        }
        DropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
            DropdownMenuItem(
                text = { Text("全部分类") },
                onClick = {
                    onSelected("")
                    expanded = false
                },
            )
            paths.forEach { path ->
                val depth = path.count { it == '/' }
                val label = "　".repeat(depth) + path.substringAfterLast('/')
                DropdownMenuItem(
                    text = { Text(label) },
                    onClick = {
                        onSelected(path)
                        expanded = false
                    },
                )
            }
        }
    }
}
