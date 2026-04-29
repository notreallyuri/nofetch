-- nofetch default configuration
-- Auto-generated on first launch.

return {
	art = {
		-- Leave empty to auto-detect your OS logo, or specify a name like "arch", "windows", etc.
		-- name = "arch",
		-- Override default logo colors (optional)
		-- colors = { "cyan", "blue" },
	},
	modules = {
		-- The title module shows user@hostname
		{ type = "title", icon = "", format = "{1} @ {2}" },

		-- Top border
		{ type = "custom", value = "    ┌─────────┐", color = "gray" },

		-- System Information
		{ type = "os", label = "OS", separator = "", format = "│ {label:blue}      │ {1}" },
		{ type = "kernel", label = "Kernel", separator = "", format = "│ {label:cyan}  │ {1}" },
		{ type = "uptime", label = "Uptime", separator = "", format = "│ {label:cyan}  │ {1}" },
		{ type = "packages", label = "Packs", separator = "", format = "│ {label:cyan}   │ {1}" },
		{ type = "shell", label = "Shell", separator = "", format = "│ {label:cyan}   │ {1}" },
		{ type = "wm", label = "WM", separator = "", format = "│ {label:cyan}      │ {1}" },

		-- Hardware Information
		{ type = "display", label = "Disp", separator = "", format = "│ {label:green}    │ {1}" },
		{ type = "cpu", label = "CPU", separator = "", format = "│ {label:red}     │ {1}" },
		{ type = "gpu", label = "GPU", separator = "", format = "│ {label:red}     │ {1}" },
		{ type = "gpu_driver", label = "Driver", separator = "", format = "│ {label:red}  │ {1}" },

		-- Usage Information (with dynamic colors based on thresholds)
		{
			type = "memory",
			label = "Mem",
			separator = "",
			format = "│ {label:magenta}     │ {1} / {2} ({3:dynamic})",
			thresholds = { 60, 80 }, -- Turns yellow at 60%, red at 80%
		},

		-- Disk usage. You can add multiple disks by specifying the `path`.
		{
			type = "disk",
			label = "Disk",
			separator = "",
			format = "│ {label:magenta}    │ {1} / {2} ({3:dynamic})",
			thresholds = { 70, 90 },
			-- For Windows, use a path like "C:\\" or "Z:\\"
			path = "/", -- Default Linux root
		},

		-- Example of adding a second disk (e.g., Home folder or Games drive)
		-- {
		--     type = "disk",
		--     label = "Home",
		--     separator = "",
		--     format = "│ {label:magenta}    │ {1} / {2} ({3:dynamic})",
		--     thresholds = { 70, 90 },
		--     path = "/home",
		-- },

		{ type = "os_age", label = "Age", separator = "", format = "│ {label:gray}     │ {1}" },

		-- Bottom border
		{ type = "custom", value = "    └─────────┘", color = "gray" },

		-- Blank line spacer
		{ type = "custom", value = "" },

		-- Color palette showcase at the bottom
		{ type = "colors", symbol = "circle" },
	},
}
