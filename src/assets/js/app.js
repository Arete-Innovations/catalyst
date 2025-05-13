document.addEventListener('DOMContentLoaded', function() {
	const csrfInput = document.querySelector('input[name="authenticity_token"]');
	if (!csrfInput || !csrfInput.value) {
		const hasRefreshed = sessionStorage.getItem('csrfRefreshed');
		if (!hasRefreshed) {
			sessionStorage.setItem('csrfRefreshed', 'true');
			location.reload();
		}
	}
});
